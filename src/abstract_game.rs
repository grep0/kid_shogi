// Traits describing abstract game

pub trait Position {
    fn possible_moves(self: &Self) -> Vec<String>;
    fn make_move(self: &Self, mv: &str) -> Option<Box<dyn Position>>;
    fn to_str(self: &Self) -> String;
    fn is_lost(self: &Self) -> bool;
    fn current_player(self: &Self) -> i32;  // actually 0 or 1
}

pub trait PositionFactory {
    fn game_name(&self) -> &str;
    fn initial(&self) -> Box<dyn Position>;
    fn from_str(&self, s: &str) -> Option<Box<dyn Position>>;
}

#[cfg(test)]
pub mod tests {
    use super::*;

    // Very simple game to test strategies
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
                if m<self.value { return None }
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
        fn is_lost(self: &Self) -> bool {
            self.value==0
        }
        fn possible_moves(self: &Self) -> Vec<String> {
            (1..=std::cmp::min(2,self.value)).into_iter().map(|v| v.to_string()).collect()
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