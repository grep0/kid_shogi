use crate::abstract_game::PositionFactory;
use crate::strategy::StrategyEngine;
use std::io::{stdin, stdout, Write};
use clap::Parser;

mod kids_shogi;
mod abstract_game;
mod strategy;
mod neuro;
mod mcts;

fn play_cmd_line(human_player: i32, strat: &mut dyn StrategyEngine) {
    let game_factory = kids_shogi::PositionFactory{};
    let mut pos = game_factory.initial();
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
                let mv = strat.choose_move(pos.as_ref());
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

#[derive(Parser)]
struct Argv {
    // Human player (0=first, 1=second)
    #[arg(short='p', long, default_value_t = 0)]
    human_player: i32,
    // Num tries for MCTS
    #[arg(long, default_value_t = 100)]
    num_tries: usize,
}

fn main() {
    let args = Argv::parse();
    let evaluator = kids_shogi::SimpleEvaluator{};
    let mut strat =
        mcts::MonteCarloTreeSearchStrategy::new(evaluator, args.num_tries, 3.0);
    play_cmd_line(args.human_player, &mut strat);
}
