#!/usr/bin/env python3
"""
Match runner for kid_shogi engines.

Usage:
  python match.py <games> <binary1> [args1...] -- <binary2> [args2...]

Example:
  python match.py 20 ./target/release/kid_shogi --num-tries 200 \
      -- ./target/release/kid_shogi --num-tries 50

Engine protocol (--engine mode):
  - Each engine runs as a persistent subprocess.
  - The match runner sends the initial FEN to the engine whose turn it is.
  - Each engine reads a FEN from stdin, makes its move, and prints either:
      • The new FEN (game continues — fed directly to the other engine), or
      • A result string: "1-0" (Sente wins), "0-1" (Gote wins), "1/2-1/2" (draw).
  - The match runner only inspects lines for the three result strings;
    everything else is forwarded verbatim to the other engine.

Game alternation:
  - Engine A plays Sente (moves first) in odd-numbered games, Gote in even ones.
"""

import subprocess
import sys

INITIAL_FEN = "gle/1c1/1C1/ELG b -"
RESULTS = {"1-0", "0-1", "1/2-1/2"}


def start_engine(cmd: list[str]) -> subprocess.Popen:
    return subprocess.Popen(
        cmd + ["--engine"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )


def send_line(proc: subprocess.Popen, line: str) -> None:
    proc.stdin.write(line + "\n")
    proc.stdin.flush()


def recv_line(proc: subprocess.Popen) -> str:
    line = proc.stdout.readline()
    if not line:
        raise RuntimeError(f"Engine {proc.args[0]} closed stdout unexpectedly")
    return line.rstrip("\n")


def play_game(cmd_a: list[str], cmd_b: list[str], a_is_sente: bool) -> str:
    """
    Play one game. Returns 'A', 'B', or 'draw'.

    Engine A is Sente when a_is_sente, otherwise Gote.
    Sente moves first; the engine that is Sente receives the initial FEN.
    """
    eng_a = start_engine(cmd_a)
    eng_b = start_engine(cmd_b)

    # sente_eng moves first; gote_eng is the other
    if a_is_sente:
        sente_eng, gote_eng = eng_a, eng_b
        sente_label, gote_label = "A", "B"
    else:
        sente_eng, gote_eng = eng_b, eng_a
        sente_label, gote_label = "B", "A"

    result = "draw"
    try:
        # Sente receives the initial position
        send_line(sente_eng, INITIAL_FEN)
        # Engines alternate: after Sente moves, Gote receives the output, etc.
        movers = [(sente_eng, sente_label), (gote_eng, gote_label)]
        mover_idx = 0  # 0 = Sente just moved, feed output to Gote
        while True:
            current_eng, current_label = movers[mover_idx]
            response = recv_line(current_eng)
            if response in RESULTS:
                if response == "1/2-1/2":
                    result = "draw"
                elif response == "1-0":
                    result = sente_label   # Sente wins
                else:  # "0-1"
                    result = gote_label    # Gote wins
                break
            # Not a result: it's the next FEN — send to the other engine
            mover_idx = 1 - mover_idx
            next_eng, _ = movers[mover_idx]
            send_line(next_eng, response)
    finally:
        for eng in (eng_a, eng_b):
            try:
                eng.stdin.close()
                eng.wait(timeout=5)
            except Exception:
                eng.kill()

    return result


def parse_args(argv):
    if len(argv) < 4 or "--" not in argv:
        print(__doc__)
        sys.exit(1)
    sep = argv.index("--")
    games = int(argv[0])
    binary1 = argv[1:sep]
    binary2 = argv[sep + 1:]
    if not binary1 or not binary2:
        print("Both binaries must be specified around '--'")
        sys.exit(1)
    return games, binary1, binary2


def main():
    games, binary1, binary2 = parse_args(sys.argv[1:])

    wins_a = draws = wins_b = 0

    for i in range(games):
        a_is_sente = (i % 2 == 0)
        try:
            result = play_game(binary1, binary2, a_is_sente)
        except RuntimeError as e:
            print(f"Game {i+1}: ERROR — {e}")
            continue

        if result == "draw":
            draws += 1
            tag = "draw"
        elif result == "A":
            wins_a += 1
            tag = "A wins"
        else:
            wins_b += 1
            tag = "B wins"

        print(f"Game {i+1:3d} (A={'Sente' if a_is_sente else 'Gote '}): {tag}")

    print()
    print(f"Results after {games} games:")
    print(f"  A wins : {wins_a}")
    print(f"  Draws  : {draws}")
    print(f"  B wins : {wins_b}")


if __name__ == "__main__":
    main()
