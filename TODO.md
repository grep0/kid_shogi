* GUI: smooth move animation; animate the player's move immediately not waiting for the server
* GUI: drag&drop interface instead of clicking source and destination
* GUI: use web/assets/board.svg as board background; flip it if the player is gote
* GUI: show the game record (list of moves played so far)

* Code organization: move strategy factory under mcts, create it in main and pass to RPC address
* Strategy: wrap MCTS into FindWinningMoveStrategys

* WEB server: serve both GUI and RPC from the same port on different addresses
* WEB server: add "remove_game" RPC call; GUI should call it when new game starts
* WEB server: add rpc logging

* MCTS: analyze the code for possible logic errors; the engine is currently very weak

* NEURO: rewrite using dfdx
* NEURO: create neuro strategy factory, the weights must be shared between game instances