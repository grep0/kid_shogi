use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rand::Rng;
use jsonrpc_core::{IoHandler, Params, Value, Error};

use crate::abstract_game::{AbstractGame, StrategyFactory};

// ── Request / response types ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct StartGameRequest {
    player: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StartGameResponse {
    game_id: String,
    position: String,
    last_move: Option<String>,
    possible_moves: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RemoveGameRequest {
    game_id: String,
}

#[derive(serde::Deserialize)]
struct MakeMoveRequest {
    game_id: String,
    #[serde(rename = "move")]
    move_: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum GameResult {
    YouWon,
    IWon,
    Draw,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MakeMoveResponse {
    position: String,
    last_move: Option<String>,
    possible_moves: Vec<String>,
    game_result: Option<GameResult>,
}

// ── Game registry ─────────────────────────────────────────────────────────────

struct GameEntry {
    #[allow(dead_code)]
    human_player: i32,
    position: String,
}

struct GameRegistry {
    games: HashMap<String, GameEntry>,
}

impl GameRegistry {
    fn new() -> Self {
        GameRegistry { games: HashMap::new() }
    }

    fn insert(&mut self, entry: GameEntry) -> String {
        let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
        self.games.insert(id.clone(), entry);
        id
    }

    fn get(&self, id: &str) -> Option<&GameEntry> {
        self.games.get(id)
    }

    fn remove(&mut self, id: &str) {
        self.games.remove(id);
    }
}

// ── Game server ───────────────────────────────────────────────────────────────

struct GameServer<PosT: AbstractGame + Send + 'static, FactoryT: StrategyFactory<PosT>> {
    registry: Mutex<GameRegistry>,
    strategy_factory: FactoryT,
    phantom_pos: std::marker::PhantomData<PosT>,
}

// SAFETY: registry is protected by Mutex; factory is Send+Sync; PhantomData
// holds no PosT data.
unsafe impl<PosT: AbstractGame + Send + 'static, FactoryT: StrategyFactory<PosT>> Send for GameServer<PosT, FactoryT> {}
unsafe impl<PosT: AbstractGame + Send + 'static, FactoryT: StrategyFactory<PosT>> Sync for GameServer<PosT, FactoryT> {}

impl<PosT: AbstractGame + Send + 'static, FactoryT: StrategyFactory<PosT>> GameServer<PosT, FactoryT> {
    fn new(strategy_factory: FactoryT) -> Self {
        GameServer {
            registry: Mutex::new(GameRegistry::new()),
            strategy_factory,
            phantom_pos: std::marker::PhantomData,
        }
    }

    fn start_game(&self, params: Params) -> Result<Value, Error> {
        let request: StartGameRequest = params.parse()
            .map_err(|e| Error::invalid_params(e.message))?;
        if request.player != 0 && request.player != 1 {
            return Err(Error::invalid_params("player must be 0 or 1"));
        }
        let mut strategy = self.strategy_factory.create();
        let (pos, last_move) = if request.player == 0 {
            (PosT::initial(), None)
        } else {
            let initial = PosT::initial();
            let mv = strategy.choose_move(&initial).unwrap();
            let new_pos = initial.make_move(&mv).unwrap();
            (new_pos, Some(mv))
        };
        let game_id = self.registry.lock().unwrap()
            .insert(GameEntry { human_player: request.player, position: pos.to_str() });
        let response = StartGameResponse {
            game_id,
            position: pos.to_str(),
            last_move,
            possible_moves: pos.possible_moves(),
        };
        Ok(serde_json::to_value(&response).unwrap())
    }

    fn remove_game(&self, params: Params) -> Result<Value, Error> {
        let request: RemoveGameRequest = params.parse()
            .map_err(|e| Error::invalid_params(e.message))?;
        self.registry.lock().unwrap().remove(&request.game_id);
        Ok(Value::Null)
    }

    fn make_move(&self, params: Params) -> Result<Value, Error> {
        let request: MakeMoveRequest = params.parse()
            .map_err(|e| Error::invalid_params(e.message))?;
        let pos_str = {
            let registry = self.registry.lock().unwrap();
            let entry = registry.get(&request.game_id)
                .ok_or_else(|| Error::invalid_params("unknown game_id"))?;
            entry.position.clone()
        };
        let mut strategy = self.strategy_factory.create();
        let pos = PosT::from_str(&pos_str).expect("registry position must be valid");
        let Some(new_pos) = pos.make_move(&request.move_) else {
            return Err(Error::invalid_params("invalid move"));
        };
        if new_pos.is_lost() {
            self.registry.lock().unwrap().remove(&request.game_id);
            let response = MakeMoveResponse {
                position: new_pos.to_str(),
                last_move: None,
                possible_moves: new_pos.possible_moves(),
                game_result: Some(GameResult::YouWon),
            };
            return Ok(serde_json::to_value(&response).unwrap());
        }
        let my_move = strategy.choose_move(&new_pos).unwrap();
        let Some(my_new_pos) = new_pos.make_move(&my_move) else {
            return Err(Error::internal_error());
        };
        let game_result = if my_new_pos.is_lost() {
            self.registry.lock().unwrap().remove(&request.game_id);
            Some(GameResult::IWon)
        } else {
            self.registry.lock().unwrap().games.get_mut(&request.game_id).unwrap().position = my_new_pos.to_str();
            None
        };
        let response = MakeMoveResponse {
            position: my_new_pos.to_str(),
            last_move: Some(my_move),
            possible_moves: my_new_pos.possible_moves(),
            game_result,
        };
        Ok(serde_json::to_value(&response).unwrap())
    }
}

pub fn create_io_handler<PosT, FactoryT>(factory: FactoryT) -> IoHandler
where
    PosT: AbstractGame + Send + 'static,
    FactoryT: StrategyFactory<PosT> + 'static,
{
    let server = Arc::new(GameServer::new(factory));
    let mut io = IoHandler::default();
    let s1 = Arc::clone(&server);
    io.add_sync_method("start_game", move |params| s1.start_game(params));
    let s2 = Arc::clone(&server);
    io.add_sync_method("make_move", move |params| s2.make_move(params));
    let s3 = Arc::clone(&server);
    io.add_sync_method("remove_game", move |params| s3.remove_game(params));
    io
}

#[cfg(test)]
pub mod tests {

use super::*;
use serde_json::Value;
use crate::{kids_shogi, mcts::MctsFactory};

fn test_io() -> IoHandler {
    static EVAL: kids_shogi::SimpleEvaluator = kids_shogi::SimpleEvaluator {};
    create_io_handler(MctsFactory::new(&EVAL, 1000, 3.0, 8))
}

#[test]
fn start_game() {
    let io = test_io();
    let request0 = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":0}, "id":1}"#;
    let response0 = io.handle_request_sync(request0).unwrap();
    println!("response0={}", response0);
    let value0 = serde_json::from_str::<Value>(&response0).unwrap();
    let resp0: StartGameResponse = serde_json::from_value(
        value0.get("result").unwrap().clone()).unwrap();
    assert_eq!(resp0.position, "gle/1c1/1C1/ELG b -");
    assert_eq!(resp0.last_move, None);
    assert_eq!(resp0.possible_moves.len(), 4); // one c, one g, two l
    assert_eq!(resp0.game_id.len(), 16);

    let request1 = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":1}, "id":2}"#;
    let response1 = io.handle_request_sync(request1).unwrap();
    let value1 = serde_json::from_str::<Value>(&response1).unwrap();
    let resp1: StartGameResponse = serde_json::from_value(
        value1.get("result").unwrap().clone()).unwrap();
    assert_eq!(resp1.position, "gle/1C1/3/ELG w C");
    assert_eq!(resp1.last_move, Some("b2b3".to_string()));
    assert_eq!(resp1.game_id.len(), 16);
}

#[test]
fn make_move() {
    let io = test_io();

    // Start as player 0 (Sente, moves first)
    let start_req = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":0}, "id":1}"#;
    let start_resp: StartGameResponse = serde_json::from_value(
        serde_json::from_str::<Value>(&io.handle_request_sync(start_req).unwrap())
            .unwrap().get("result").unwrap().clone()).unwrap();
    let game_id = &start_resp.game_id;
    assert!(start_resp.possible_moves.contains(&"b2b3".to_string()));

    // Human plays b2b3 (chicken forward)
    let move_req = format!(
        r#"{{"jsonrpc": "2.0", "method":"make_move", "params":{{"game_id":"{game_id}", "move":"b2b3"}}, "id":2}}"#);
    let move_val = serde_json::from_str::<Value>(&io.handle_request_sync(&move_req).unwrap()).unwrap();
    assert!(move_val.get("error").is_none(), "unexpected error: {move_val}");
    let move_resp: MakeMoveResponse = serde_json::from_value(
        move_val.get("result").unwrap().clone()).unwrap();

    // Server must reply with a valid position and a move
    assert!(move_resp.last_move.is_some());
    assert!(move_resp.game_result.is_none(), "game should not be over yet");
    assert!(!move_resp.possible_moves.is_empty());

    // Position stored in registry must have advanced (second make_move uses it)
    let move2_req = format!(
        r#"{{"jsonrpc": "2.0", "method":"make_move", "params":{{"game_id":"{game_id}", "move":"{}"}}, "id":3}}"#,
        move_resp.possible_moves[0]);
    let move2_val = serde_json::from_str::<Value>(&io.handle_request_sync(&move2_req).unwrap()).unwrap();
    assert!(move2_val.get("error").is_none(), "unexpected error on second move: {move2_val}");
}

#[test]
fn invalid_move_rejected() {
    let io = test_io();
    let start_req = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":0}, "id":1}"#;
    let start_resp: StartGameResponse = serde_json::from_value(
        serde_json::from_str::<Value>(&io.handle_request_sync(start_req).unwrap())
            .unwrap().get("result").unwrap().clone()).unwrap();
    let game_id = &start_resp.game_id;

    let move_req = format!(
        r#"{{"jsonrpc": "2.0", "method":"make_move", "params":{{"game_id":"{game_id}", "move":"b1b4"}}, "id":2}}"#);
    let move_val = serde_json::from_str::<Value>(&io.handle_request_sync(&move_req).unwrap()).unwrap();
    assert!(move_val.get("error").is_some());
}

#[test]
fn unknown_game_id_rejected() {
    let io = test_io();
    let request = r#"{"jsonrpc": "2.0", "method":"make_move", "params":{"game_id":"deadbeefdeadbeef", "move":"b2b3"}, "id":1}"#;
    let response = io.handle_request_sync(request).unwrap();
    let value = serde_json::from_str::<Value>(&response).unwrap();
    assert!(value.get("error").is_some());
}

}
