// Traits describing abstract game

use std::any::Any;

pub trait Position {
    fn possible_moves(self: &Self) -> Vec<String>;
    fn make_move(self: &Self, mv: &str) -> Option<Box<dyn Position>>;
    fn to_str(self: &Self) -> String;
    fn is_lost(self: &Self) -> bool;
    fn current_player(self: &Self) -> i32;  // actually 0 or 1
    fn encode(self: &Self) -> Vec<f64>;  // for neuro
    fn pretty_print(self: &Self) -> String;
    fn as_any(self: &Self) -> &dyn Any;  // for downcasting
    fn clone_to_box(self: &Self) -> Box<dyn Position>;
}

pub trait PositionFactory {
    fn game_name(&self) -> &str;
    fn initial(&self) -> Box<dyn Position>;
    fn from_str(&self, s: &str) -> Option<Box<dyn Position>>;
}

pub trait Evaluator {
    fn evaluate_position(&self, pos: &dyn Position) -> f64;
    // Return saturation value for this evaluator; if ±saturation is returned,
    // evaluator believes that the position is won/lost
    fn saturation(&self) -> f64;
}

#[cfg(test)]
pub mod tests {
    use super::*;

    // Very simple game to test strategies
    // Start with a heap of K stones, player can take one or two stones
    // One who takes the last stone wins
     
    #[derive(Clone)]
    pub(crate) struct OneTwoGamePosition {
        value: i32,
        player: i32,
    }
    impl Position for OneTwoGamePosition {
        fn current_player(self: &Self) -> i32 {
            return self.player;
        }
        fn make_move(self: &Self, mv: &str) -> Option<Box<dyn Position>> {
            if let Ok(m) = mv.parse::<i32>() {
                if m!=1 && m!=2 { return None }
                if m>self.value { return None }
                let bx: Box::<dyn Position> = Box::new(
                    OneTwoGamePosition{value: self.value-m, player: 1-self.player });
                return Some(bx);
            } else {
                None  // parse error
            }
        }
        fn to_str(self: &Self) -> String {
            format!("{} {}", self.value, self.player)
        }
        fn pretty_print(self: &Self) -> String {
            self.to_str()
        }
        fn is_lost(self: &Self) -> bool {
            self.value==0
        }
        fn possible_moves(self: &Self) -> Vec<String> {
            (1..=std::cmp::min(2,self.value)).into_iter().map(|v| v.to_string()).collect()
        }
        fn encode(self: &Self) -> Vec<f64> {
            vec![self.value as f64]
        }
        fn as_any(self: &Self) -> &dyn Any {
            self
        }
        fn clone_to_box(self: &Self) -> Box<dyn Position> {
            Box::new(self.clone())
        }
    }

    pub(crate) struct OneTwoGameFactory {}
    impl PositionFactory for OneTwoGameFactory {
        fn game_name(&self) -> &str {
            "OneTwoGame"
        }
        fn initial(&self) -> Box<dyn Position> {
            return Box::new(OneTwoGamePosition{value:10, player:0})
        }
        fn from_str(&self, s: &str) -> Option<Box<dyn Position>> {
            let parts = s.split(' ').collect::<Vec<_>>();
            let bx: Box<dyn Position> = Box::new(OneTwoGamePosition{
                value: parts[0].parse().unwrap(),
                player: parts[1].parse().unwrap()
            });
            Some(bx)
        }
    }
}