use jsonrpc_core::{IoHandler, Params, Value, Error};
use jsonrpc_http_server::Server;
use serde;
use serde_json::ser;

use crate::{abstract_game::AbstractGame, kids_shogi as ks};

#[derive(serde::Deserialize)]
struct StartGameRequest {
    player: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StartGameResponse {
    position: String,
    last_move: Option<String>,
    possible_moves: Vec<String>,
}

#[derive(serde::Deserialize)]
struct MakeMoveRequest {
    position: String,
    move_: String,
}

#[derive(serde::Serialize)]
struct MakeMoveResponse {
    position: String,
    last_move: String,
    possible_moves: Vec<String>,
    game_result: Option<String>,
}

fn create_io_handler() -> IoHandler<()> {
    let mut io = IoHandler::default();
    io.add_sync_method("start_game", move |params: Params| {
        let request: StartGameRequest = params.parse()
            .map_err(|e| Error::invalid_params(e.message))?;
        if request.player!=0 && request.player!=1 {
            return Err(Error::invalid_params("player must be 0 or 1"))
        }
        let (pos, last_move) = 
            if request.player==0 {
                (ks::Position::initial(), None)
            } else {
                let last_move = String::from("b2b3");
                (ks::Position::initial().make_move(&last_move).unwrap(), Some(last_move))
            };
        let response = StartGameResponse {
            position: pos.to_str(),
            last_move: last_move,
            possible_moves: pos.possible_moves(),
        };
        Ok(serde_json::to_value(&response).unwrap())
    });
    io.add_sync_method("make_move", move |params: Params| {
        let request: MakeMoveRequest = params.parse()
            .map_err(|e| Error::invalid_params(e.message))?;
        let Some(pos) = ks::Position::from_str(&request.position)
        else {
            return Err(Error::invalid_params("invalid position"))
        };
        let Some(new_pos) = pos.make_move(&request.move_)
        else {
            return Err(Error::invalid_params("invalid move"))
        };
        let response = MakeMoveResponse {
            position: new_pos.to_str(),
            last_move: request.move_,
            possible_moves: new_pos.possible_moves(),
            game_result: if new_pos.is_lost() {
                Some(match new_pos.current_player() {
                    0 => "Gote",
                    1 => "Sente",
                    _ => panic!("impossible"),
                }.to_string())
            } else {
                None
            },
        };
        Ok(serde_json::to_value(&response).unwrap())
    });
    io
}

#[cfg(test)]
pub mod tests {

use jsonrpc_core::response;

use super::*;

#[test]
fn start_game() {
    let io = create_io_handler();
    let request0 = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":0}, "id":1}"#;
    let response0 = io.handle_request_sync(request0).unwrap();
    println!("response0={}", response0);
    let value0 = serde_json::from_str::<Value>(&response0).unwrap();
    let resp0 : StartGameResponse = serde_json::from_value(
        value0.get("result").unwrap().clone()).unwrap();
    assert_eq!(resp0.position, "gle/1c1/1C1/ELG b -");
    assert_eq!(resp0.last_move, None);
    assert_eq!(resp0.possible_moves.len(), 4); // one c, one g, two l

    let request1 = r#"{"jsonrpc": "2.0", "method":"start_game", "params":{"player":1}, "id":2}"#;
    let response1 = io.handle_request_sync(request1).unwrap();
    let value1 = serde_json::from_str::<Value>(&response1).unwrap();
    let resp1 : StartGameResponse = serde_json::from_value(
        value1.get("result").unwrap().clone()).unwrap();
    assert_eq!(resp1.position, "gle/1C1/3/ELG w C");
    assert_eq!(resp1.last_move, Some("b2b3".to_string()));
}

}