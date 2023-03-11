use abstract_game::PositionFactory;

use crate::{abstract_game::Position, strategy::StrategyEngine};
use std::io::{self, stdin, stdout, Write};

mod kids_shogi;
mod abstract_game;
mod strategy;
mod neuro;
mod mcts;

fn main() {
    let evaluator = kids_shogi::SimpleEvaluator{};
    let mut strat = mcts::MonteCarloTreeSearchStrategy::new(evaluator, 30);

    let game_factory = kids_shogi::PositionFactory{};
    let mut pos = game_factory.initial();
    while !pos.is_lost() {
        println!("{:?}", pos.to_str());
        let mv = match pos.current_player() {
            0 => {
                loop {
                    print!("Sente move> ");
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
                    }
                }
            }
            1 => {
                let mv = strat.choose_move(pos.as_ref());
                println!("Gote move> {}", mv.clone().unwrap_or("???".to_string()));
                mv
            }
            _ => panic!("what a player")
        };
        if mv.is_none() {
            println!("Game ended for some weird reason");
            break
        }
        pos = pos.make_move(&mv.unwrap()).expect("must be a valid move");
    }
}
