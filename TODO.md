* WEB server: serve both GUI and RPC from the same port on different addresses
* WEB server: add "remove_game" RPC call; GUI should call it when a new game starts
* WEB server: add rpc logging

* MCTS: analyze the code for possible logic errors; the engine is currently very weak

* GUI: drag&drop interface instead of clicking source and destination

* NEURO: rewrite using dfdx
* NEURO: create neuro strategy factory, the weights must be shared between game instances