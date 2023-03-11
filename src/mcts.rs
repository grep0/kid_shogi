// Monte Carlo tree search

use std::collections::HashMap;

use crate::abstract_game as ag;
use crate::strategy;

pub struct MonteCarloTreeSearchStrategy<Eval: ag::Evaluator> {
    num_tries: usize,
    softness: f64,
    eval: Eval,
}

impl<Eval: ag::Evaluator> MonteCarloTreeSearchStrategy<Eval> {
    pub fn new(eval: Eval, num_tries: usize, softness: f64) -> Self {
        return MonteCarloTreeSearchStrategy{eval: eval, num_tries: num_tries, softness: softness}
    }

    fn choose_move_internal(&mut self, depth: i32, pos: &dyn ag::Position, num_tries: usize) -> (Option<String>, f64) {
        if pos.is_lost() { return (None, -self.eval.saturation() )}
        if num_tries <=1 { return (None, self.eval.evaluate_position(pos)) }
        let mut softmax = strategy::SoftMaxStrategy::new(&self.eval, self.softness);
        let choices = softmax.multi_choose_move(pos, num_tries-1);
        dbg!(depth, pos.to_str(), pos.possible_moves(), &choices);
        let scores: HashMap<String, f64> = choices.into_iter().map(|(mv, count)| {
            let pos1 = pos.make_move(&mv).unwrap();
            let (_, mvscore) = self.choose_move_internal(depth+1, pos1.as_ref(), count);
            let sc = if pos.current_player() == pos1.as_ref().current_player() {mvscore} else {-mvscore};
            (mv, sc)
        }).collect();
        dbg!(depth, &scores);
        // the move is the best move, and score is softmin of all the scores
        let best =
            scores.iter().max_by(|&u, &v| u.1.partial_cmp(v.1).unwrap()).unwrap().0;
        let total = scores.iter().fold(0.0, |acc, x| acc + (-x.1).exp());
        (Some(best.clone()), -(total/scores.len() as f64).ln())
    }
}

impl<Eval: ag::Evaluator> strategy::StrategyEngine for MonteCarloTreeSearchStrategy<Eval> {
    fn choose_move(&mut self, pos: &dyn ag::Position) -> Option<String> {
        let (opt_mv, _) = self.choose_move_internal(0, pos, self.num_tries);
        opt_mv
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{abstract_game::{tests as agt, PositionFactory}, strategy::{self, StrategyEngine}};

    use super::MonteCarloTreeSearchStrategy;

    // This is a somewhat probabilistic test but it succesfully solves OneTwoGame
    #[test]
    fn mcts() {
        let fac = agt::OneTwoGameFactory{};
        let pos = fac.from_str("11 0").unwrap();
        let eval = strategy::OneStepEvaluator{};
        let mut strat = MonteCarloTreeSearchStrategy::new(
            eval, 255, 1.0);
        let mv = strat.choose_move(pos.as_ref());
        assert_eq!(mv.unwrap(), "2");
    }
}