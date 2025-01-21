use std::io::{self, Read, Write};
use std::fs;
use std::marker::PhantomData;

use nn;
use serde;

use crate::abstract_game::{self as ag, Evaluator};
use crate::strategy::{self,StrategyEngine};
use crate::mcts::MonteCarloTreeSearchStrategy;

pub struct NeuroEvaluator<PosT: ag::NeuroPosition> {
    nn: nn::NN,
    phantom_data: PhantomData<PosT>,
}

impl <PosT: ag::NeuroPosition> NeuroEvaluator<PosT> {
    pub fn new() -> Self {
        // we need factory to find the valency of input layers
        let input_deg = PosT::encode_length();
        // hardcode internal layers for now
        Self{
            nn: nn::NN::new(&[input_deg as u32, 1024, 256, 16, 1]),
            phantom_data: PhantomData
        }
    }

    fn save(self: &Self, outf: &mut fs::File) -> Result<(), std::io::Error> {
        let encoded = self.nn.to_json();
        outf.write(encoded.as_bytes())?;
        Ok(())
    }

    fn load(inf: &mut fs::File) -> Result<Self, std::io::Error> {
        let mut buf = Vec::<u8>::new();
        inf.read_to_end(&mut buf)?;
        match String::from_utf8(buf) {
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Ok(sbuf) => Ok(Self {
                nn: nn::NN::from_json(&sbuf),
                phantom_data: PhantomData
            })
        }
    }
}

impl <PosT: ag::NeuroPosition> ag::Evaluator<PosT> for NeuroEvaluator<PosT> {
    fn evaluate_position(self: &Self, pos: &PosT) -> f64 {
        let res = self.nn.run(&pos.encode()[..]);
        res[0].tanh()
    }
    fn saturation(self: &Self) -> f64 {
        1.0
    }
}

type Example = (Vec<f64>, Vec<f64>);

fn random_games<PosT: ag::NeuroPosition, StratT: StrategyEngine<PosT>>(
        strat: &mut StratT,
        num_games: usize, max_moves: usize, decay: f64) -> Vec<Example> {
    let mut examples = Vec::<Example>::new();
    for _ in 0..num_games {
        // forward: make n moves with current evaluator
        let mut propagation: Vec<(Vec<f64>, i32, f64)> = Vec::<(Vec<f64>, i32, f64)>::new();
        let mut current_pos = PosT::initial();
        for _ in 0..max_moves {
            let mv = strat.choose_move(&current_pos);
            if mv.is_none() { break }
            let encoded_pos = current_pos.encode();
            propagation.push((encoded_pos, current_pos.current_player(), 0.0));
            current_pos = current_pos.make_move(&mv.unwrap()).unwrap();
            println!("  current_pos: {:?}", current_pos.to_str());
        }
        let onestep = strategy::OneStepEvaluator::<PosT>::new();
        let final_eval = onestep.evaluate_position(&current_pos);
        println!("final eval {}", final_eval);
        for i in 0..propagation.len() {
            let decayed_eval = final_eval * decay.powi((propagation.len()-i) as i32);
            propagation[i].2 = if propagation[i].1==current_pos.current_player() {decayed_eval} else {-decayed_eval}
        }
        //println!("propagation: {:?}", propagation);
        examples.append(
            &mut propagation.into_iter().map(|(pos, _, eval)| (pos, vec![eval])).collect()
        );
    };
    examples
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct TrainParameters {
    mtsc_tries: usize,
    softness: f64,
    num_games: usize,
    game_depth: usize,
    score_decay: f64,
    train_once_epochs: usize,
    train_sessions: usize
}

impl Default for TrainParameters {
    fn default() -> Self {
        TrainParameters {
            mtsc_tries: 20,
            softness: 3.0,
            num_games: 10,
            game_depth: 10,
            score_decay: 0.8,
            train_once_epochs: 100,
            train_sessions: 10
        }
    }
}

fn train_once<PosT: ag::NeuroPosition>(eval: &mut NeuroEvaluator<PosT>, params: &TrainParameters) {
    println!("Collecting examples...");
    let examples = {
        let eval_ref = &*eval;
        let mut strat = MonteCarloTreeSearchStrategy::new(eval_ref, params.mtsc_tries, params.softness);
        random_games(&mut strat, params.num_games, params.game_depth, params.score_decay)
    };
    println!("Training...");
    eval.nn.train(&examples).halt_condition(nn::HaltCondition::Epochs(params.train_once_epochs as u32)).go();
}

#[allow(dead_code)]
pub fn train<PosT: ag::NeuroPosition>(eval: &mut NeuroEvaluator<PosT>, params: &TrainParameters) {
    for _ in 0..params.train_sessions {
        train_once(eval, params);
    }
}

pub fn load_model<PosT: ag::NeuroPosition>(filename: &str) -> Result<NeuroEvaluator<PosT>, io::Error> {
    let mut file = fs::File::open(filename)?;
    return NeuroEvaluator::load(&mut file)
}

pub fn load_params(filename: &str) -> Result<TrainParameters, io::Error> {
    let file = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(file);
    let params = serde_json::from_reader(&mut reader)?;
    Ok(params)
}

pub fn save_model<PosT: ag::NeuroPosition>(
        nn: &NeuroEvaluator<PosT>, filename: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(filename)?;
    return nn.save(&mut file)
}

pub fn save_params(params: &TrainParameters, filename: &str) -> Result<(), io::Error> {
    let file = fs::File::create(filename)?;
    let writer = io::BufWriter::new(&file);
    match serde_json::to_writer(writer, params) {
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        Ok(()) => Ok(())
    }
}
