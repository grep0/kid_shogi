// Basic strategy engine
use super::abstract_game::Position;

use rand::Rng;

pub trait StrategyEngine {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String>; 
}

struct RandomMoveStrategy {
    rng : rand::rngs::StdRng,
}

impl StrategyEngine for RandomMoveStrategy {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String> {
        let moves = pos.possible_moves();
        if moves.is_empty() {
            None
        } else {
            let n = self.rng.gen_range(0..moves.len());
            Some(moves[n].clone())
        }
    }
}

struct FindWinningMoveStrategy<FollowupStrategy: StrategyEngine> {
    followup: FollowupStrategy,
}

impl<F: StrategyEngine> StrategyEngine for FindWinningMoveStrategy<F> {
    fn choose_move(&mut self, pos: &dyn Position) -> Option<String> {
        let moves = pos.possible_moves();
        let n = moves.iter().position(
            |mv| pos.make_move(mv).and_then(|pos1| Some(pos1.is_lost())).unwrap_or(false));
        if n.is_none() {
            self.followup.choose_move(pos)
        } else {
            Some(moves[n.unwrap()].clone())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::abstract_game::{tests as agt, PositionFactory};
    use rand::SeedableRng;

    use super::{RandomMoveStrategy, StrategyEngine, FindWinningMoveStrategy};

    #[test]
    fn random_move_strategy() {
        let fac = agt::OneTwoGameFactory{};
        let g = fac.from_str("5 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(32)
        };
        assert_eq!(strategy.choose_move(g.as_ref()).unwrap(), "1")
    }

    #[test]
    fn random_move_strategy_no_moves() {
        let fac = agt::OneTwoGameFactory{};
        let g = fac.from_str("0 0").unwrap();
        let mut strategy = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(42)
        };
        assert!(strategy.choose_move(g.as_ref()).is_none())
    }

    #[test]
    fn find_winning_move_strategy() {
        let fac = agt::OneTwoGameFactory{};
        let followup = RandomMoveStrategy {
            rng: rand::rngs::StdRng::seed_from_u64(32)
        };
        let mut strategy = FindWinningMoveStrategy{ followup: followup };

        // No immediately winning move, uses followup
        let g = fac.from_str("5 0").unwrap();
        assert_eq!(strategy.choose_move(g.as_ref()).unwrap(), "1");

        // Now with immediately winning move
        let g2 = fac.from_str("2 0").unwrap();
        assert_eq!(strategy.choose_move(g2.as_ref()).unwrap(), "2");
    }

}