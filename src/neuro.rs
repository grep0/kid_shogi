use nn;

use crate::abstract_game::{self as ag, Evaluator};
use crate::strategy::{self,StrategyEngine};

struct NeuroEvaluator {
    nn: nn::NN,
}

impl NeuroEvaluator {
    fn new(factory: &dyn ag::PositionFactory) -> NeuroEvaluator {
        // we need factory to find the valency of input layer
        let input_deg = factory.initial().as_ref().encode().len();
        // hardcode internal layers for now
        NeuroEvaluator{
            nn: nn::NN::new(&[input_deg as u32, 1024, 256, 16, 1]),
        }
    }
}

impl ag::Evaluator for &NeuroEvaluator {
    fn evaluate_position(self: &Self, pos: &dyn ag::Position) -> f64 {
        let res = self.nn.run(&pos.encode()[..]);
        res[0].tanh()
    }
    fn saturation(self: &Self) -> f64 {
        1.0
    }
}

fn random_game(factory: &dyn ag::PositionFactory, eval: &NeuroEvaluator, max_moves: usize, decay: f64) {
    let softmax = strategy::SoftMaxStrategy::new(&eval, 1.0);
    let mut strat = strategy::FindWinningMoveStrategy::new(softmax);
    let mut current_pos = factory.initial();
    let mut propagation = Vec::<(Vec<f64>, i32, f64)>::new();
    // forward: make n moves with current evaluator
    for _ in 0..max_moves {
        let mv = strat.choose_move(current_pos.as_ref());
        if mv.is_none() { break }
        let encoded_pos = current_pos.as_ref().encode();
        propagation.push((encoded_pos, current_pos.current_player(), 0.0));
        current_pos = current_pos.as_ref().make_move(&mv.unwrap()).unwrap();
    }
    let onestep = strategy::OneStepEvaluator{};
    let final_eval = onestep.evaluate_position(current_pos.as_ref());
    for i in 0..propagation.len() {
        let decayed_eval = final_eval * decay.powi((propagation.len()-i) as i32);
        propagation[i].2 = if propagation[i].1==current_pos.current_player() {decayed_eval} else {-decayed_eval}
    }
}