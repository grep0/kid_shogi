// Traits describing abstract game

pub trait AbstractGame : Sized + Clone {
    /// Compact integer type used as a collision-free map key for positions.
    /// Choose the smallest type that fits all reachable positions for the game
    /// (e.g. u32 for tiny games, u64 for kid_shogi).
    /// There is intentionally no `from_hash` — hashes are write-only keys.
    type PositionHash: Eq + std::hash::Hash + Copy;

    fn possible_moves(self: &Self) -> Vec<String>;
    fn make_move(self: &Self, mv: &str) -> Option<Self>;
    fn to_str(self: &Self) -> String;
    /// Encode this position as a `PositionHash`. Must be injective over all
    /// reachable positions: distinct positions must produce distinct hashes.
    fn to_hash(self: &Self) -> Self::PositionHash;
    fn is_lost(self: &Self) -> bool;
    fn current_player(self: &Self) -> i32;  // actually 0 or 1
    fn pretty_print(self: &Self) -> String;

    fn initial() -> Self;
    fn from_str(s: &str) -> Option<Self>;
}

pub trait NeuroPosition : AbstractGame {
    const ENCODE_LENGTH: usize;
    fn encode(self: &Self) -> Vec<f64>;
}

pub trait StrategyFactory<PosT: AbstractGame + Send + 'static>: Send + Sync {
    fn create(&self) -> Box<dyn crate::strategy::StrategyEngine<PosT>>;
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
        type PositionHash = u32;
        fn to_str(self: &Self) -> String {
            format!("{} {}", self.value, self.player)
        }
        fn to_hash(self: &Self) -> u32 {
            (self.value as u32) << 1 | (self.player as u32)
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
        const ENCODE_LENGTH: usize = 1;
        fn encode(self: &Self) -> Vec<f64> {
            vec![self.value as f64]
        }
    }
}