use crate::strategy::StrategyEngine;
use std::io::{stdin, stdout, Write};
use abstract_game::{AbstractGame, Evaluator, NeuroPosition};
use clap::Parser;

mod kids_shogi;
mod abstract_game;
mod strategy;
mod neuro;
mod mcts;
mod rpc;
mod static_server;

type GamePosition = kids_shogi::KidsShogiGame;
const ENCODE_LEN: usize = <GamePosition as NeuroPosition>::ENCODE_LENGTH;
type NeuroEval = neuro::NeuroEvaluator<GamePosition, ENCODE_LEN>;

fn play_cmd_line<EngineT: StrategyEngine<GamePosition>>(human_player: i32, strat: &mut EngineT) {
    let mut pos = GamePosition::initial();
    while !pos.is_lost() {
        println!("{}", pos.pretty_print());
        let mv = match pos.current_player() {
            v if v==human_player => {
                loop {
                    print!("Human move> ");
                    stdout().flush().expect("oops flush");
                    let mut buf = String::new();
                    stdin().read_line(&mut buf).expect("failed to read line");
                    let mv = buf.trim();
                    if mv.is_empty() {
                        break None
                    }
                    let new_pos_or = pos.make_move(mv);
                    if new_pos_or.is_some() {
                        break Some(mv.to_string())
                    } else {
                        println!("Possible moves: {}", pos.possible_moves().join(" "));
                    }
                }
            }
            _ => {
                let mv = strat.choose_move(&pos);
                println!("Machine move> {}", mv.clone().unwrap_or("???".to_string()));
                mv
            }
        };
        if mv.is_none() {
            println!("Game ended for some weird reason");
            break
        }
        pos = pos.make_move(&mv.unwrap()).expect("must be a valid move");
    }
    if pos.is_lost() {
        let winner = match pos.current_player() {
            0 => "Gote",
            1 => "Sente",
            _ => panic!("impossible"),
        };
        println!("{} wins!", winner)
    } else {
        println!("Game terminated (was it draw?)");
    }
}

#[derive(clap::Parser)]
struct Argv {
    // Human player (0=first, 1=second, 2=play with self)
    #[arg(short='p', long, default_value_t = 0)]
    human_player: i32,
    // Num tries for MCTS
    #[arg(long, default_value_t = 1000)]
    num_tries: usize,
    // Exploration softness for MCTS
    #[arg(long, default_value_t = 3.0)]
    softness: f64,
    // Max tree depth per MCTS rollout
    #[arg(long, default_value_t = 8)]
    max_depth: i32,
    // Path to neural network weights; if given, uses neuro evaluator instead of SimpleEvaluator
    #[arg(long)]
    model_file: Option<String>,
    #[arg(short='t', long)]
    train: bool,
    // Number of training epochs to run (default 1)
    #[arg(long, default_value_t = 1)]
    max_epochs: usize,
    // Engine mode: loop reading FEN lines, printing moves
    #[arg(short='e', long)]
    engine: bool,
    // Run JSON-RPC HTTP server instead of CLI game
    #[arg(short='s', long)]
    server: bool,
    // Address for the combined HTTP server (GUI + RPC at /rpc)
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,
    // Directory to serve static web UI files from
    #[arg(long, default_value = "src/web")]
    web_root: std::path::PathBuf,
}

fn run_engine_loop<EvalT: Evaluator<GamePosition>>(eval: &EvalT, args: &Argv) {
    use std::io::BufRead;
    const MAX_HALF_MOVES: usize = 100;
    let mut half_moves: usize = 0;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let fen = line.expect("read error");
        if half_moves >= MAX_HALF_MOVES {
            println!("1/2-1/2");
            break;
        }
        let pos = GamePosition::from_str(&fen).expect("invalid FEN");
        let mut strat = mcts::MonteCarloTreeSearchStrategy::new(
            eval, args.num_tries, args.softness, args.max_depth);
        let mv = strat.choose_move(&pos).expect("no moves");
        let new_pos = pos.make_move(&mv).expect("chosen move must be valid");
        half_moves += 1;
        if new_pos.is_lost() {
            let result = if new_pos.current_player() == 0 { "0-1" } else { "1-0" };
            println!("{}", result);
            break;
        }
        println!("{}", new_pos.to_str());
    }
}

fn run_server<EvalT: Evaluator<GamePosition> + Send + Sync + 'static>(
    eval: EvalT, args: &Argv,
) {
    let eval_ref: &'static EvalT = Box::leak(Box::new(eval));
    let io = rpc::create_io_handler(
        mcts::MctsFactory::new(eval_ref, args.num_tries, args.softness, args.max_depth));
    let addr = args.listen.parse().expect("invalid listen address");
    println!("Serving at http://{} (GUI: /, RPC: /rpc)", args.listen);
    static_server::serve(io, args.web_root.clone(), addr);
}

fn main() {
    let args = Argv::parse();

    // ── Training ──────────────────────────────────────────────────────────────
    if args.train {
        let model_file = args.model_file.as_deref().unwrap_or("ks.model");
        let params_file = format!("{}.params", model_file);
        let mut nn: NeuroEval =
            neuro::load_model(model_file)
                .map(|m| { println!("Loaded model from {}", model_file); m })
                .unwrap_or_else(|_| { println!("No model at {}, starting fresh", model_file); NeuroEval::new() });
        let params = neuro::load_params(&params_file)
            .map(|p| { println!("Loaded params from {}", params_file); p })
            .unwrap_or_else(|_| { println!("Using default train parameters"); neuro::TrainParameters::default() });
        println!("Parameters: {:?}", params);
        println!("Max epochs: {}", args.max_epochs);
        let eval = kids_shogi::SimpleEvaluator{};
        for epoch in 0..args.max_epochs {
            neuro::train_epoch(&eval, &mut nn, &params, epoch, model_file)
                .expect("training failed");
        }
        neuro::save_model(&nn, model_file).unwrap();
        neuro::save_params(&params, &params_file).unwrap();
        println!("Final model saved to {}", model_file);
        return;
    }

    // ── Engine loop ───────────────────────────────────────────────────────────
    if args.engine {
        if let Some(ref model_file) = args.model_file {
            let nn: NeuroEval = neuro::load_model(model_file)
                .expect("failed to load model");
            eprintln!("Engine: using neuro model from {}", model_file);
            run_engine_loop(&nn, &args);
        } else {
            run_engine_loop(&kids_shogi::SimpleEvaluator{}, &args);
        }
        return;
    }

    // ── HTTP server ───────────────────────────────────────────────────────────
    if args.server {
        if let Some(ref model_file) = args.model_file {
            let nn: NeuroEval = neuro::load_model(model_file)
                .expect("failed to load model");
            println!("Server: using neuro model from {}", model_file);
            run_server(nn, &args);
        } else {
            run_server(kids_shogi::SimpleEvaluator{}, &args);
        }
        return;
    }

    // ── CLI game ──────────────────────────────────────────────────────────────
    if let Some(ref model_file) = args.model_file {
        let nn: NeuroEval = neuro::load_model(model_file)
            .expect("failed to load model");
        let mut strat = mcts::MonteCarloTreeSearchStrategy::new(
            &nn, args.num_tries, args.softness, args.max_depth);
        play_cmd_line(args.human_player, &mut strat);
    } else {
        let mut strat = mcts::MonteCarloTreeSearchStrategy::new(
            &kids_shogi::SimpleEvaluator{}, args.num_tries, args.softness, args.max_depth);
        play_cmd_line(args.human_player, &mut strat);
    }
}
