// Traits describing abstract game

pub trait AbstractGame : Sized + Clone {
    fn possible_moves(self: &Self) -> Vec<String>;
    fn make_move(self: &Self, mv: &str) -> Option<Self>;
    fn to_str(self: &Self) -> String;
    fn is_lost(self: &Self) -> bool;
    fn current_player(self: &Self) -> i32;  // actually 0 or 1
    fn pretty_print(self: &Self) -> String;

    fn initial() -> Self;
    fn from_str(s: &str) -> Option<Self>;
}

pub trait NeuroPosition : AbstractGame {
    fn encode(self: &Self) -> Vec<f64>;  // for neuro
    fn encode_length() -> usize;
}

pub trait Evaluator<PosT: AbstractGame> {
    fn evaluate_position(&self, pos: &PosT) -> f64;
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
    pub(crate) struct OneTwoGame {
        value: i32,
        player: i32,
    }
    impl AbstractGame for OneTwoGame {
        fn current_player(self: &Self) -> i32 {
            return self.player;
        }
        fn make_move(self: &Self, mv: &str) -> Option<Self> {
            if let Ok(m) = mv.parse::<i32>() {
                if m!=1 && m!=2 { return None }
                if m>self.value { return None }
                Some(Self{ value: self.value-m, player: 1-self.player })
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

        fn initial() -> Self {
            return Self{value:10, player:0}
        }
        fn from_str(s: &str) -> Option<Self> {
            let parts = s.split(' ').collect::<Vec<_>>();
            let pos= Self{
                value: parts[0].parse().unwrap(),
                player: parts[1].parse().unwrap()
            };
            Some(pos)
        }
    }

    impl NeuroPosition for OneTwoGame {
        fn encode(self: &Self) -> Vec<f64> {
            vec![self.value as f64]
        }
        fn encode_length() -> usize {
            1
        }
    }
}