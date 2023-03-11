This is an experiment of creating a game engine for [Dōbutsu shōgi][game], a simple
Shogi variant played on a 3x4 field.

Currently implemented: [Monte Carlo Tree Search][mcts] against a simple greedy evaluator.
Doesn't play very well, but ok for the baseline.

Planned:
* Pair MCTS with a neural network evaluator, implementing some [Reinforcement learning][rl]
* Make a web server and write some JS client side

[game]: https://en.wikipedia.org/wiki/D%C5%8Dbutsu_sh%C5%8Dgi
[mcts]: https://en.wikipedia.org/wiki/Monte_Carlo_tree_search
[rl]: https://en.wikipedia.org/wiki/Reinforcement_learning