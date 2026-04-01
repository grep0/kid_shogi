use std::collections::HashMap;
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
use rand::seq::SliceRandom;

use crate::abstract_game::{self as ag, Evaluator};
use crate::mcts::MonteCarloTreeSearchStrategy;
use crate::strategy::{FindWinningMoveStrategy, StrategyEngine};

type Dev = Cpu;

// IN → 1024 → ReLU → 256 → ReLU → 16 → ReLU → 1 → Tanh
type MlpConfig<const IN: usize> = (
    (Linear<IN, 1024>, ReLU, Linear<1024, 256>, ReLU),
    (Linear<256, 16>,  ReLU, Linear<16, 1>,     Tanh),
);

// ── NeuroEvaluator ────────────────────────────────────────────────────────────

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
        self.model.forward(input).array()[0] as f64
    }

    fn saturation(&self) -> f64 { 1.0 }
}

// ── TrainParameters ───────────────────────────────────────────────────────────

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct TrainParameters {
    /// Number of self-play games per epoch
    pub games_per_epoch: usize,
    /// MCTS rollouts per move during self-play
    pub mcts_tries: usize,
    /// MCTS softmax temperature
    pub mcts_softness: f64,
    /// MCTS tree depth cap
    pub mcts_max_depth: i32,
    /// Ply limit per game; beyond this the game is a draw
    pub max_game_depth: usize,
    /// Per-ply score decay from game end (e.g. 0.95)
    pub score_decay: f64,
    /// Max positions sampled from database for training
    pub training_subset: usize,
    /// Mini-batch size (Adam updates per example within batch)
    pub batch_size: usize,
    /// Training passes over the sampled subset
    pub training_epochs: usize,
}

impl Default for TrainParameters {
    fn default() -> Self {
        TrainParameters {
            games_per_epoch: 50,
            mcts_tries: 1000,
            mcts_softness: 3.0,
            mcts_max_depth: 8,
            max_game_depth: 50,
            score_decay: 0.95,
            training_subset: 5000,
            batch_size: 64,
            training_epochs: 20,
        }
    }
}

// ── Self-play ─────────────────────────────────────────────────────────────────

enum PlayResult {
    /// The given player index lost
    Win { loser: i32 },
    Draw,
}

/// Play one game using `eval` wrapped in FindWinningMove + MCTS.
/// Returns (hash, encoding, score) for every position visited;
/// score = ±decay^(distance_from_end), or 0.0 for draws.
fn play_game<PosT, EvalT>(
    eval: &EvalT,
    params: &TrainParameters,
) -> (Vec<(PosT::PositionHash, Vec<f64>, f64)>, PlayResult)
where
    PosT: ag::NeuroPosition,
    EvalT: ag::Evaluator<PosT>,
{
    let mcts = MonteCarloTreeSearchStrategy::new(
        eval, params.mcts_tries, params.mcts_softness, params.mcts_max_depth);
    let mut strat = FindWinningMoveStrategy::new(mcts);

    // (hash, encoding, player_at_pos)
    let mut history: Vec<(PosT::PositionHash, Vec<f64>, i32)> = Vec::new();
    let mut pos = PosT::initial();

    loop {
        if pos.is_lost() {
            let loser = pos.current_player();
            let n = history.len();
            let scored = history.into_iter().enumerate().map(|(i, (hash, enc, player))| {
                let dist = (n - i) as i32;
                let sign: f64 = if player == loser { -1.0 } else { 1.0 };
                (hash, enc, sign * params.score_decay.powi(dist))
            }).collect();
            return (scored, PlayResult::Win { loser });
        }
        if history.len() >= params.max_game_depth {
            let scored = history.into_iter()
                .map(|(hash, enc, _)| (hash, enc, 0.0))
                .collect();
            return (scored, PlayResult::Draw);
        }
        history.push((pos.to_hash(), pos.encode(), pos.current_player()));
        let Some(mv) = strat.choose_move(&pos) else {
            // No moves without is_lost — shouldn't happen, treat as draw
            let scored = history.into_iter()
                .map(|(hash, enc, _)| (hash, enc, 0.0))
                .collect();
            return (scored, PlayResult::Draw);
        };
        pos = pos.make_move(&mv).unwrap();
    }
}

// ── Database ──────────────────────────────────────────────────────────────────

struct DbEntry {
    encoding: Vec<f64>,
    score_sum: f64,
    count:     u32,
}

impl DbEntry {
    fn avg_score(&self) -> f64 { self.score_sum / self.count as f64 }
}

type Database<H> = HashMap<H, DbEntry>;

fn db_insert<H: Eq + std::hash::Hash>(
    db: &mut Database<H>,
    hash: H,
    encoding: Vec<f64>,
    score: f64,
) {
    let entry = db.entry(hash).or_insert(DbEntry { encoding, score_sum: 0.0, count: 0 });
    entry.score_sum += score;
    entry.count += 1;
}

fn generate_database<PosT, EvalT>(
    eval: &EvalT,
    params: &TrainParameters,
) -> Database<PosT::PositionHash>
where
    PosT: ag::NeuroPosition,
    EvalT: ag::Evaluator<PosT>,
{
    let mut db: Database<PosT::PositionHash> = HashMap::new();
    let mut total_plies = 0usize;
    let mut sente_wins = 0usize; // loser = player 1 (Gote)
    let mut gote_wins  = 0usize; // loser = player 0 (Sente)
    let mut draws      = 0usize;

    println!("  Generating {} self-play games (mcts_tries={}, max_depth={})...",
        params.games_per_epoch, params.mcts_tries, params.mcts_max_depth);

    for g in 0..params.games_per_epoch {
        let (positions, result) = play_game::<PosT, EvalT>(eval, params);
        let plies = positions.len();
        total_plies += plies;

        let outcome_str = match &result {
            PlayResult::Win { loser: 0 } => { gote_wins  += 1; "Gote wins " }
            PlayResult::Win { .. }       => { sente_wins += 1; "Sente wins" }
            PlayResult::Draw             => { draws       += 1; "draw      " }
        };

        for (hash, enc, score) in positions {
            db_insert(&mut db, hash, enc, score);
        }

        println!("    Game {:3}/{}: {:3} plies, {} | DB: {} unique positions",
            g + 1, params.games_per_epoch, plies, outcome_str, db.len());
    }

    let avg_score = db.values().map(|e| e.avg_score()).sum::<f64>() / db.len() as f64;
    let avg_abs   = db.values().map(|e| e.avg_score().abs()).sum::<f64>() / db.len() as f64;

    println!("  Self-play complete: {} total plies → {} unique positions",
        total_plies, db.len());
    println!("  Results: Sente-wins={} Gote-wins={} draws={}",
        sente_wins, gote_wins, draws);
    println!("  Score stats: mean={:.4}  mean(|score|)={:.4}", avg_score, avg_abs);

    db
}

// ── Training ──────────────────────────────────────────────────────────────────

fn train_on_database<PosT, const IN: usize>(
    db: &Database<PosT::PositionHash>,
    model: &mut NeuroEvaluator<PosT, IN>,
    params: &TrainParameters,
)
where
    PosT: ag::NeuroPosition,
{
    let mut rng = rand::thread_rng();

    // Sample up to training_subset positions
    let all_entries: Vec<&DbEntry> = db.values().collect();
    let n_samples = params.training_subset.min(all_entries.len());
    let sampled: Vec<&&DbEntry> = all_entries
        .choose_multiple(&mut rng, n_samples)
        .collect();

    println!("  Training on {}/{} positions  (batch_size={}, epochs={})",
        n_samples, db.len(), params.batch_size, params.training_epochs);

    // Pre-convert to f32 once
    let mut training_data: Vec<([f32; IN], f32)> = sampled.into_iter().map(|e| {
        let arr: [f32; IN] = e.encoding.iter()
            .map(|&x| x as f32)
            .collect::<Vec<_>>()
            .try_into()
            .expect("encoding length must equal IN");
        (arr, e.avg_score() as f32)
    }).collect();

    let mut grads = model.model.alloc_grads();
    let mut opt   = Adam::new(&model.model, Default::default());

    for epoch in 0..params.training_epochs {
        training_data.shuffle(&mut rng);
        let mut loss_sum  = 0f64;
        let mut n_updates = 0usize;

        for batch in training_data.chunks(params.batch_size) {
            for (input_arr, target_val) in batch {
                let input:  Tensor<Rank1<IN>, f32, Dev> = model.dev.tensor(*input_arr);
                let target: Tensor<Rank1<1>,  f32, Dev> = model.dev.tensor([*target_val]);
                let pred = model.model.forward_mut(input.trace(grads));
                let loss = mse_loss(pred, target);
                loss_sum  += loss.array() as f64;
                n_updates += 1;
                grads = loss.backward();
                opt.update(&mut model.model, &grads).unwrap();
                model.model.zero_grads(&mut grads);
            }
        }

        let avg_loss = loss_sum / n_updates as f64;
        // Log every epoch for visibility; mark first and last clearly
        let marker = if epoch == 0 { " <start>" } else if epoch + 1 == params.training_epochs { " <end>" } else { "" };
        println!("    Epoch {:3}/{}: avg_loss={:.6}{}",
            epoch + 1, params.training_epochs, avg_loss, marker);
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run one training epoch:
/// 1. Generate self-play games using `self_play_eval`
/// 2. Aggregate into an in-memory position database
/// 3. Train `model` on a sample of that database
/// 4. Save checkpoint to `{model_file}.epoch{epoch}`
pub fn train_epoch<PosT, EvalT, const IN: usize>(
    self_play_eval: &EvalT,
    model: &mut NeuroEvaluator<PosT, IN>,
    params: &TrainParameters,
    epoch: usize,
    model_file: &str,
) -> io::Result<()>
where
    PosT: ag::NeuroPosition,
    EvalT: ag::Evaluator<PosT>,
{
    println!("\n=== Epoch {} ===", epoch);
    println!("--- Self-play phase ---");
    let db = generate_database::<PosT, EvalT>(self_play_eval, params);

    println!("--- Training phase ---");
    train_on_database::<PosT, IN>(&db, model, params);

    let checkpoint = format!("{}.epoch{}", model_file, epoch);
    model.save(&checkpoint)?;
    println!("  Checkpoint saved → {}", checkpoint);

    Ok(())
}

// ── File I/O ──────────────────────────────────────────────────────────────────

pub fn load_model<PosT: ag::NeuroPosition, const IN: usize>(
    path: &str,
) -> io::Result<NeuroEvaluator<PosT, IN>> {
    let mut eval = NeuroEvaluator::new();
    eval.load_weights(path)?;
    Ok(eval)
}

pub fn save_model<PosT: ag::NeuroPosition, const IN: usize>(
    eval: &NeuroEvaluator<PosT, IN>,
    path: &str,
) -> io::Result<()> {
    eval.save(path)
}

pub fn load_params(path: &str) -> io::Result<TrainParameters> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn save_params(params: &TrainParameters, path: &str) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, params)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
