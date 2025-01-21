// Monte Carlo tree search

use std::collections::{HashMap,HashSet};
use std::marker::PhantomData;

use crate::abstract_game::{self as ag};
use crate::strategy::{self, StrategyEngine};

struct Node {
    #[allow(dead_code)]
    pos: String,
    parents: HashSet<String>,
    evaluation: f64,
    visits: usize,  // number of visits so far
    reward: f64,    // total reward collected
    children: HashMap<String, String>,  // move->pos
    is_populated: bool,
}

struct MCTSState<PosT: ag::AbstractGame> {
    nodes: HashMap<String, Node>,
    phantom_data: PhantomData<PosT>,
}

fn clamp(v: f64) -> f64 {
    if v< -1.0 { -1.0 } else if v>1.0 { 1.0 } else { v }
}

impl<PosT: ag::AbstractGame> MCTSState<PosT> {
    fn make_node<EvalT: ag::Evaluator<PosT>>(&mut self, pos: &PosT, parent: Option<&PosT>, evaluator: &EvalT) {
        let pos_str = pos.to_str();
        if let Some(existing_node) = self.nodes.get_mut(&pos_str) {
            if let Some(p) = parent {
                existing_node.parents.insert(p.to_str());
            }
            return
        }
        let n = Node{
            pos: pos_str.clone(),
            parents: parent.into_iter().map(ag::AbstractGame::to_str).collect(),
            evaluation: clamp(evaluator.evaluate_position(pos) / evaluator.saturation()),
            visits: 0,
            reward: 0.0,
            children: HashMap::new(),
            is_populated: false,
        };
        self.nodes.insert(pos_str, n);
    }

    fn populate_children<EvalT: ag::Evaluator<PosT>>(&mut self, pos: &PosT, evaluator: &EvalT) {
        let pos_str = pos.to_str();
        let parent_node = self.nodes.get(&pos_str).expect("parent node must exist");
        if parent_node.is_populated { return }
        let moves = pos.possible_moves();
        //eprintln!("From pos {} possible moves {:?}", pos_str, moves);
        let children =
            moves.into_iter().map(|mv| {
                let new_pos = pos.make_move(&mv).unwrap();
                self.make_node(&new_pos, Some(pos), evaluator);
                (mv, new_pos.to_str())
            }).collect();
        let parent_mut = self.nodes.get_mut(&pos_str).unwrap();
        parent_mut.children = children;
        parent_mut.is_populated = true;
    }

    fn update_node(&mut self, pos: &PosT, reward: f64) {
        let pos_str = pos.to_str();
        let node = self.nodes.get_mut(&pos_str).expect("node must exist");
        node.visits+=1;
        node.reward+=reward;
        //eprintln!("Pos={} visits={} reward={}", pos_str, node.visits, node.reward);
    }

    #[allow(dead_code)]
    fn print_move_tree(&self, pos: &PosT, depth: i32, indent: i32) {
        let indents = String::from_utf8(vec![b' '; indent as usize]).unwrap();
        pos.possible_moves().into_iter().for_each(|mv| {
            let new_pos = pos.make_move(&mv).unwrap();
            if let Some(node) = self.nodes.get(&new_pos.to_str()) {
                eprintln!("{}{} {}({}) #{}", &indents, mv, node.reward, node.evaluation, node.visits);
                if depth>0 {
                    self.print_move_tree(&new_pos, depth-1, indent+4);
                }
            } else {
                eprintln!("{}{} not vidited", &indents, mv);
            }
        });

    }

    fn choose_best_by_reward(&self, pos: &PosT) -> Option<String> {
        let moves = pos.possible_moves();
        let c = moves.into_iter().map(|mv| {
            let new_pos = pos.make_move(&mv).unwrap();
            let reward = self.nodes.get(&new_pos.to_str()).unwrap().reward;
            //eprintln!("mv={} visits={} reward={}", mv, self.nodes.get(&new_pos.to_str()).unwrap().visits, reward);
            (mv, reward)
        }).min_by(|a, b| a.1.total_cmp(&b.1)).clone();
        match c {
            Some((mv, _)) => Some(mv),
            None => None
        }
    }
}

impl<PosT: ag::AbstractGame> ag::Evaluator<PosT> for MCTSState<PosT> {
    fn saturation(self: &Self) -> f64 {
        return 1.0
    }
    fn evaluate_position(self: &Self, pos: &PosT) -> f64 {
        let pos_str = pos.to_str();
        if let Some(node) = self.nodes.get(&pos_str) {
            let parent_visits: usize = node.parents.iter().map(
                |p| self.nodes.get(p).unwrap().visits).sum();
            let explore_bonus = (parent_visits as f64 + 1.0).ln() / ((node.visits+1) as f64);
            let eval_bonus = node.evaluation / ((node.visits+1) as f64);
            let avg_reward = if node.visits>0 {node.reward/(node.visits as f64)} else {0.0};
            //eprintln!("Eval pos {} : {} eval_bonus {} explore_bonus {}", pos_str, avg_reward, eval_bonus, explore_bonus);
            avg_reward - eval_bonus - explore_bonus
        } else {
            //eprintln!("No node for pos {}", pos_str);
            return 0.0
        }
    }
}

pub struct MonteCarloTreeSearchStrategy<'a, PosT: ag::AbstractGame, EvalT: ag::Evaluator<PosT>> {
    num_tries: usize,
    softness: f64,
    max_depth: i32,
    eval: &'a EvalT,
    phantom_data: PhantomData<PosT>,
}

impl<'a, PosT: ag::AbstractGame, EvalT: ag::Evaluator<PosT>> MonteCarloTreeSearchStrategy<'a, PosT, EvalT> {
    pub fn new(eval: &'a EvalT, num_tries: usize, softness: f64) -> Self {
        return MonteCarloTreeSearchStrategy{eval: eval, num_tries: num_tries, softness: softness, max_depth: 8, phantom_data: PhantomData}
    }

    fn walk_once(&mut self, start_pos: &PosT, state: &mut MCTSState<PosT>) {
        let mut pos = start_pos.clone();
        let mut track = Vec::new();
        let mut track_moves = Vec::new();
        while track.len() < self.max_depth.try_into().unwrap() {
            if pos.is_lost() {
                break
            }
            state.populate_children(&pos, self.eval);
            let mut softmax =
                strategy::SoftMaxStrategy::new(&*state, self.softness);
            if let Some(choice) = softmax.choose_move(&pos) {
                let pos1 = pos.make_move(&choice).unwrap();
                //eprintln!("move={} pos1={}", choice, pos1.to_str());
                track.push(pos);
                track_moves.push(choice);
                pos = pos1
            } else {
                break
            }
        }
        let player_final = pos.current_player();
        let ev_final = self.eval.evaluate_position(&pos)/self.eval.saturation();
        //eprintln!("moves: {:?} player_final: {} ev_final: {}", track_moves, player_final, ev_final);
        track.push(pos);
        track.into_iter().rev().for_each(|p| {
            let ev = if p.current_player() == player_final {ev_final} else {-ev_final};
            state.update_node(&p, ev)
        })
    }
}

impl<'a, PosT: ag::AbstractGame, EvalT: ag::Evaluator<PosT>> strategy::StrategyEngine<PosT> for MonteCarloTreeSearchStrategy<'a, PosT, EvalT> {
    fn choose_move(&mut self, pos: &PosT) -> Option<String> {
        let mut state = MCTSState{ nodes: HashMap::new(), phantom_data: PhantomData };
        state.make_node(pos,None, self.eval);
        for _ in 1..self.num_tries {
            self.walk_once(pos, &mut state)
        }
        //state.print_move_tree(pos, 2, 0);
        state.choose_best_by_reward(pos)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{abstract_game::{tests as agt, AbstractGame}, strategy::{self, StrategyEngine}};

    use super::MonteCarloTreeSearchStrategy;

    // This is a somewhat probabilistic test but it succesfully solves OneTwoGame
    #[test]
    fn smoke() {
        let pos = agt::OneTwoGame::from_str("8 0").unwrap();
        let eval = strategy::OneStepEvaluator::<agt::OneTwoGame>::new();
        let mut strat = MonteCarloTreeSearchStrategy::new(
            &eval, 32, 3.0);
        let mv = strat.choose_move(&pos);
        assert_eq!(mv.unwrap(), "2");
    }
}