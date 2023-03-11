use std::collections::HashMap;

// Basic strategy engine
use super::abstract_game::{Position, Evaluator};

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rand::distributions::WeightedIndex;

pub trait StrategyEngine {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String>; 
}

pub struct RandomMoveStrategy {
    rng : StdRng,
}

impl StrategyEngine for RandomMoveStrategy {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String> {
        let moves = pos.possible_moves();
        if moves.is_empty() {
            None
        } else {
            let n = self.rng.gen_range(0..moves.len());
            Some(moves[n].clone())
        }
    }
}

pub struct FindWinningMoveStrategy<FollowupStrategy: StrategyEngine> {
    followup: FollowupStrategy,
}

impl<F: StrategyEngine> FindWinningMoveStrategy<F> {
    pub fn new(f: F) -> Self {
        FindWinningMoveStrategy { followup: f }
    }
}

impl<F: StrategyEngine> StrategyEngine for FindWinningMoveStrategy<F> {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String> {
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

pub struct OneStepEvaluator {}

impl OneStepEvaluator {
    const SATURATION : f64 = 10.0;
}

impl Evaluator for OneStepEvaluator {
    fn saturation(&self) -> f64 {
        Self::SATURATION
    }
    fn evaluate_position(&self, pos: &dyn Position) -> f64 {
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

pub struct SoftMaxStrategy<'a, Eval: Evaluator> {
    eval: &'a Eval,
    softness: f64,  // "softness" coefficient - how much we trust the evaluator
    rng: StdRng,
}

impl<'a, E: Evaluator> SoftMaxStrategy<'a, E> {
    pub fn new(e: &'a E,softness: f64) -> Self {
        SoftMaxStrategy{
            eval: e,
            softness: softness,
            rng: StdRng::from_entropy(),
        }
    }

    pub fn multi_choose_move(&mut self, pos: &dyn Position, count: usize) -> HashMap<String, usize> {
        let moves = pos.possible_moves();
        if moves.is_empty() { return HashMap::new() }
        let values = moves.iter().map(
            |mv| -self.eval.evaluate_position(pos.make_move(mv).unwrap().as_ref()))
            .map(|v| (v*self.softness).exp())
            .collect::<Vec<f64>>();
        let sum = values.iter().fold(0.0, |acc,x| acc+x);
        let weights = values.iter().map(|v| ((v * 1000000.0)/sum) as i32).collect::<Vec<_>>();
        dbg!(&moves, &weights);
        let wi = WeightedIndex::new(&weights[..]).unwrap();
        let mut res = HashMap::<String, usize>::new();
        for _ in 0..count {
            let k = moves[self.rng.sample(&wi)].clone();
            if let Some(it) = res.get_mut(&k) {
                *it += 1
            } else {
                res.insert(k, 1);
            }
        }
        res
    }
}

impl<'a, E: Evaluator> StrategyEngine for SoftMaxStrategy<'a, E> {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String> {
        let multi = self.multi_choose_move(pos, 1);
        multi.keys().into_iter().next().map(|k| k.clone())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::abstract_game::{tests as agt, PositionFactory};
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn random_move_strategy() {
        let fac = agt::OneTwoGameFactory{};
        let g = fac.from_str("5 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: StdRng::seed_from_u64(32)
        };
        assert_eq!(strategy.choose_move(g.as_ref()).unwrap(), "1")
    }

    #[test]
    fn random_move_strategy_no_moves() {
        let fac = agt::OneTwoGameFactory{};
        let g = fac.from_str("0 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(42)
        };
        assert!(strategy.choose_move(g.as_ref()).is_none())
    }

    #[test]
    fn find_winning_move_strategy() {
        let fac = agt::OneTwoGameFactory{};
        let followup = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(32)
        };
        let mut strategy = FindWinningMoveStrategy{ followup: followup };

        // No immediately winning move, uses followup
        let g = fac.from_str("5 0").unwrap();
        assert_eq!(strategy.choose_move(g.as_ref()).unwrap(), "1");

        // Now with immediately winning move
        let g2 = fac.from_str("2 0").unwrap();
        assert_eq!(strategy.choose_move(g2.as_ref()).unwrap(), "2");
    }

    #[test]
    fn one_step_evaluator() {
        let fac = agt::OneTwoGameFactory{};
        let eval = OneStepEvaluator{};
        let lost = fac.from_str("0 0").unwrap();
        assert_eq!(eval.evaluate_position(lost.as_ref()), -eval.saturation());
        let won = fac.from_str("2 0").unwrap();
        assert_eq!(eval.evaluate_position(won.as_ref()), eval.saturation());
        let undecided = fac.initial();
        assert_eq!(eval.evaluate_position(undecided.as_ref()), 0.0);
    }
}