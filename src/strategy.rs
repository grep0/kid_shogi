use std::marker::PhantomData;

// Basic strategy engine
use super::abstract_game as ag;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rand::distributions::WeightedIndex;

pub trait StrategyEngine<PosT: ag::AbstractGame> {
    fn choose_move(&mut self, pos: &PosT) -> Option<String>; 
}

pub struct RandomMoveStrategy {
    rng : StdRng,
}

impl<PosT: ag::AbstractGame> StrategyEngine<PosT> for RandomMoveStrategy {
    fn choose_move(&mut self, pos: &PosT) -> Option<String> {
        let moves = pos.possible_moves();
        if moves.is_empty() {
            None
        } else {
            let n = self.rng.gen_range(0..moves.len());
            Some(moves[n].clone())
        }
    }
}

pub struct FindWinningMoveStrategy<PosT: ag::AbstractGame, FollowupStrategy: StrategyEngine<PosT>> {
    followup: FollowupStrategy,
    pos_type: PhantomData<PosT>,
}

impl<PosT: ag::AbstractGame, F: StrategyEngine<PosT>> FindWinningMoveStrategy<PosT, F> {
    pub fn new(f: F) -> Self {
        FindWinningMoveStrategy { followup: f, pos_type: PhantomData }
    }
}

impl<PosT: ag::AbstractGame, F: StrategyEngine<PosT>> StrategyEngine<PosT> for FindWinningMoveStrategy<PosT, F> {
    fn choose_move(&mut self, pos: &PosT) -> Option<String> {
        let moves = pos.possible_moves();
        let n = moves.iter().position(
            |mv| pos.make_move(mv).and_then(|pos1| Some(pos1.is_lost())).unwrap_or(false));
        if n.is_none() {
            self.followup.choose_move(pos)
        } else {
            Some(moves[n.unwrap()].clone())
        }
    }
}

pub struct OneStepEvaluator<PosT: ag::AbstractGame> {
    pos_type: PhantomData<PosT>,
}

impl<PosT: ag::AbstractGame> OneStepEvaluator<PosT> {
    pub fn new() -> Self {
        OneStepEvaluator{ pos_type: PhantomData }
    }

    const SATURATION : f64 = 10.0;
}
impl<PosT: ag::AbstractGame> ag::Evaluator<PosT> for OneStepEvaluator<PosT> {
    fn saturation(&self) -> f64 {
        Self::SATURATION
    }
    fn evaluate_position(&self, pos: &PosT) -> f64 {
        if pos.is_lost() {
            return -Self::SATURATION
        }
        let moves = pos.possible_moves();
        if moves.iter().any(|mv| pos.make_move(&mv).and_then(
            |pos1| Some(pos1.is_lost())).unwrap_or(false)) {
            return Self::SATURATION
        }
        0.0
    }
}

pub struct SoftMaxStrategy<'a, PosT: ag::AbstractGame, Eval: ag::Evaluator<PosT>> {
    eval: &'a Eval,
    softness: f64,  // "softness" coefficient - how much we trust the evaluator
    rng: StdRng,
    pos_type: PhantomData<PosT>,
}

impl<'a, PosT: ag::AbstractGame, E: ag::Evaluator<PosT>> SoftMaxStrategy<'a, PosT, E> {
    pub fn new(e: &'a E,softness: f64) -> Self {
        SoftMaxStrategy{
            eval: e,
            softness: softness,
            rng: StdRng::from_entropy(),
            pos_type: PhantomData
        }
    }
}

impl<'a, PosT: ag::AbstractGame, E: ag::Evaluator<PosT>> StrategyEngine<PosT> for SoftMaxStrategy<'a, PosT, E> {
    fn choose_move(&mut self, pos: &PosT) -> Option<String> {
        let moves = pos.possible_moves();
        if moves.is_empty() { return None }
        let values = moves.iter().map(
            |mv| -self.eval.evaluate_position(&pos.make_move(mv).unwrap()))
            .map(|v| (v*self.softness).exp())
            .collect::<Vec<f64>>();
        let sum = values.iter().fold(0.0, |acc,x| acc+x);
        let weights = values.iter().map(|v| ((v * 1000000.0)/sum) as i32).collect::<Vec<_>>();
        let wi = WeightedIndex::new(&weights[..]).unwrap();
        let smpl = self.rng.sample(wi);
        //eprintln!("moves={:?} weights={:?} chosen={}", moves, weights, moves[smpl]);
        Some(moves[smpl].clone())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::abstract_game::{tests as agt, Evaluator};
    use ag::AbstractGame;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn random_move_strategy() {
        let g = agt::OneTwoGame::from_str("5 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: StdRng::seed_from_u64(32)
        };
        assert_eq!(strategy.choose_move(&g).unwrap(), "1")
    }

    #[test]
    fn random_move_strategy_no_moves() {
        let g = agt::OneTwoGame::from_str("0 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(42)
        };
        assert!(strategy.choose_move(&g).is_none())
    }

    #[test]
    fn find_winning_move_strategy() {
        let followup = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(32)
        };
        let mut strategy = FindWinningMoveStrategy::new(followup);

        // No immediately winning move, uses followup
        let g = agt::OneTwoGame::from_str("5 0").unwrap();
        assert_eq!(strategy.choose_move(&g).unwrap(), "1");

        // Now with immediately winning move
        let g2 = agt::OneTwoGame::from_str("2 0").unwrap();
        assert_eq!(strategy.choose_move(&g2).unwrap(), "2");
    }

    #[test]
    fn one_step_evaluator() {
        let eval = OneStepEvaluator::<agt::OneTwoGame>::new();
        let lost = agt::OneTwoGame::from_str("0 0").unwrap();
        assert_eq!(eval.evaluate_position(&lost), -eval.saturation());
        let won = agt::OneTwoGame::from_str("2 0").unwrap();
        assert_eq!(eval.evaluate_position(&won), eval.saturation());
        let undecided = agt::OneTwoGame::initial();
        assert_eq!(eval.evaluate_position(&undecided), 0.0);
    }
}