use std::io;
use std::marker::PhantomData;

use dfdx::{
    losses::mse_loss,
    nn::builders::*,
    optim::{Adam, Optimizer},
    prelude::*,
    shapes::Rank1,
    tensor::Cpu,
    tensor_ops::Backward,
};

use crate::abstract_game::{self as ag, Evaluator};
use crate::mcts::MonteCarloTreeSearchStrategy;
use crate::strategy::{self, StrategyEngine};

type Dev = Cpu;

// IN → 1024 → ReLU → 256 → ReLU → 16 → ReLU → 1 → Tanh
// Split into two 4-tuples because dfdx implements Module for tuples up to 6.
type MlpConfig<const IN: usize> = (
    (Linear<IN, 1024>, ReLU, Linear<1024, 256>, ReLU),
    (Linear<256, 16>, ReLU, Linear<16, 1>, Tanh),
);

pub struct NeuroEvaluator<PosT: ag::NeuroPosition, const IN: usize> {
    dev: Dev,
    model: <MlpConfig<IN> as BuildOnDevice<Dev, f32>>::Built,
    _phantom: PhantomData<PosT>,
}

impl<PosT: ag::NeuroPosition, const IN: usize> NeuroEvaluator<PosT, IN> {
    pub fn new() -> Self {
        let dev = Dev::default();
        let model = dev.build_module::<MlpConfig<IN>, f32>();
        Self { dev, model, _phantom: PhantomData }
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        self.model.save_safetensors(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))
    }

    pub fn load_weights(&mut self, path: &str) -> io::Result<()> {
        self.model.load_safetensors(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))
    }
}

impl<PosT: ag::NeuroPosition, const IN: usize> ag::Evaluator<PosT> for NeuroEvaluator<PosT, IN> {
    fn evaluate_position(&self, pos: &PosT) -> f64 {
        let arr: [f32; IN] = pos.encode().into_iter()
            .map(|x| x as f32)
            .collect::<Vec<_>>()
            .try_into()
            .expect("encode() length must equal IN");
        let input: Tensor<Rank1<IN>, f32, Dev> = self.dev.tensor(arr);
        let output = self.model.forward(input);
        output.array()[0] as f64
    }

    fn saturation(&self) -> f64 { 1.0 }
}

// ── Training ──────────────────────────────────────────────────────────────────

type Example = (Vec<f64>, f64);

fn random_games<PosT: ag::NeuroPosition, StratT: StrategyEngine<PosT>>(
    strat: &mut StratT,
    num_games: usize,
    max_moves: usize,
    decay: f64,
) -> Vec<Example> {
    let mut examples = Vec::new();
    for _ in 0..num_games {
        let mut propagation: Vec<(Vec<f64>, i32)> = Vec::new();
        let mut pos = PosT::initial();
        for _ in 0..max_moves {
            let Some(mv) = strat.choose_move(&pos) else { break };
            propagation.push((pos.encode(), pos.current_player()));
            pos = pos.make_move(&mv).unwrap();
        }
        let final_eval = strategy::OneStepEvaluator::<PosT>::new().evaluate_position(&pos);
        let final_player = pos.current_player();
        for (i, (encoded, player)) in propagation.iter().enumerate() {
            let steps_from_end = (propagation.len() - i) as i32;
            let target = if *player == final_player { final_eval } else { -final_eval };
            examples.push((encoded.clone(), target * decay.powi(steps_from_end)));
        }
    }
    examples
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct TrainParameters {
    pub mcts_tries: usize,
    pub softness: f64,
    pub max_depth: i32,
    pub num_games: usize,
    pub game_depth: usize,
    pub score_decay: f64,
    pub train_once_epochs: usize,
    pub train_sessions: usize,
}

impl Default for TrainParameters {
    fn default() -> Self {
        TrainParameters {
            mcts_tries: 20,
            softness: 3.0,
            max_depth: 8,
            num_games: 10,
            game_depth: 50,
            score_decay: 0.9,
            train_once_epochs: 100,
            train_sessions: 10,
        }
    }
}

fn train_once<PosT: ag::NeuroPosition + 'static, const IN: usize>(
    evaluator: &mut NeuroEvaluator<PosT, IN>,
    params: &TrainParameters,
) {
    println!("Collecting {} games...", params.num_games);
    let examples = {
        // SAFETY: evaluator lives for the duration of this block.
        let eval_ref: &'static NeuroEvaluator<PosT, IN> =
            unsafe { &*(evaluator as *const _) };
        let mut strat = MonteCarloTreeSearchStrategy::new(
            eval_ref, params.mcts_tries, params.softness, params.max_depth);
        random_games::<PosT, _>(&mut strat, params.num_games, params.game_depth, params.score_decay)
    };
    println!("Training on {} examples...", examples.len());

    let mut grads = evaluator.model.alloc_grads();
    let mut opt = Adam::new(&evaluator.model, Default::default());

    for epoch in 0..params.train_once_epochs {
        let mut total_loss = 0f32;
        for (input_vec, target_val) in &examples {
            let arr: [f32; IN] = input_vec.iter()
                .map(|&x| x as f32)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let input: Tensor<Rank1<IN>, f32, Dev> = evaluator.dev.tensor(arr);
            let target: Tensor<Rank1<1>, f32, Dev> = evaluator.dev.tensor([*target_val as f32]);

            let pred = evaluator.model.forward_mut(input.trace(grads));
            let loss = mse_loss(pred, target);
            total_loss += loss.array();
            grads = loss.backward();
            opt.update(&mut evaluator.model, &grads).unwrap();
            evaluator.model.zero_grads(&mut grads);
        }
        if (epoch + 1) % 10 == 0 {
            println!("  epoch {}/{}: avg loss {:.5}",
                epoch + 1, params.train_once_epochs,
                total_loss / examples.len() as f32);
        }
    }
}

pub fn train<PosT: ag::NeuroPosition + 'static, const IN: usize>(
    evaluator: &mut NeuroEvaluator<PosT, IN>,
    params: &TrainParameters,
) {
    for session in 0..params.train_sessions {
        println!("=== Session {}/{} ===", session + 1, params.train_sessions);
        train_once(evaluator, params);
    }
}

pub fn load_model<PosT: ag::NeuroPosition, const IN: usize>(path: &str) -> io::Result<NeuroEvaluator<PosT, IN>> {
    let mut eval = NeuroEvaluator::new();
    eval.load_weights(path)?;
    Ok(eval)
}

pub fn save_model<PosT: ag::NeuroPosition, const IN: usize>(eval: &NeuroEvaluator<PosT, IN>, path: &str) -> io::Result<()> {
    eval.save(path)
}

pub fn load_params(path: &str) -> io::Result<TrainParameters> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn save_params(params: &TrainParameters, path: &str) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer(writer, params).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
