use crate::strategy::StrategyEngine;
use std::io::{stdin, stdout, Write};
use abstract_game::{AbstractGame, Evaluator};
use clap::Parser;

mod kids_shogi;
mod abstract_game;
mod strategy;
// mod neuro;
mod mcts;
mod rpc;
mod static_server;

type GamePosition = kids_shogi::KidsShogiGame;

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
    #[arg(long)]
    model_file: Option<String>,
    #[arg(short='t', long)]
    train: bool,
    // Run JSON-RPC HTTP server instead of CLI game
    #[arg(short='s', long)]
    server: bool,
    // Address for the JSON-RPC API
    #[arg(long, default_value = "127.0.0.1:3030")]
    listen: String,
    // Address for the static web UI server
    #[arg(long, default_value = "127.0.0.1:8080")]
    web_listen: String,
    // Directory to serve static web UI files from
    #[arg(long, default_value = "src/web")]
    web_root: std::path::PathBuf,
}

fn play_with_evaluator<EvalT: Evaluator<GamePosition>>(eval: &EvalT, args: &Argv) {
    let mut strat = mcts::MonteCarloTreeSearchStrategy::new(
        eval, args.num_tries, 3.0);
    play_cmd_line(args.human_player, &mut strat);
}

fn main() {
    let args = Argv::parse();
    if args.train || args.model_file.is_some() {
        unimplemented!("neural network support is temporarily disabled");
    }
    if args.server {
        use jsonrpc_http_server::{ServerBuilder, DomainsValidation, AccessControlAllowOrigin};

        let web_addr = args.web_listen.parse().expect("invalid web-listen address");
        let web_root = args.web_root.clone();
        std::thread::spawn(move || static_server::serve(web_root, web_addr));
        println!("Web UI at http://{}", args.web_listen);

        let rpc_addr = args.listen.parse().expect("invalid listen address");
        let io = rpc::create_io_handler(args.num_tries);
        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Any]))
            .start_http(&rpc_addr)
            .expect("failed to start RPC server");
        println!("RPC API at http://{}", args.listen);
        server.wait();
        return;
    }
    // TODO: when re-enabling neuro, restore mod neuro and the train/model_file branches:
    // if args.train {
    //     let model_file = args.model_file.unwrap();
    //     let params_file = model_file.clone() + ".params";
    //     let mut nn = neuro::load_model(&model_file)
    //         .unwrap_or(neuro::NeuroEvaluator::<GamePosition>::new());
    //     let params = neuro::load_params(&params_file).unwrap_or(neuro::TrainParameters::default());
    //     neuro::train(&mut nn, &params);
    //     neuro::save_model(&nn, &model_file).unwrap();
    //     neuro::save_params(&params, &params_file).unwrap();
    // } else if let Some(model_file) = &args.model_file {
    //     let neuro_eval = neuro::load_model(&model_file).unwrap();
    //     play_with_evaluator(&neuro_eval, &args);
    // } else {
    play_with_evaluator(&kids_shogi::SimpleEvaluator{}, &args);
    // }
}
