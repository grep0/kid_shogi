#!/usr/bin/env python3
"""
Match runner for kid_shogi engines.

Two-player match:
  python match.py [-j N] <games> <binary1> [args1...] -- <binary2> [args2...]

Round-robin tournament (3+ engines, separated by --):
  python match.py [-j N] <games_per_pair> <binary1> [args1...] -- <binary2> [args2...] -- ...

Options:
  -j N / --jobs N   Run up to N games in parallel (default: 1)

Example:
  python match.py 20 ./kid_shogi --num-tries 100 \
      -- ./kid_shogi --num-tries 500 \
      -- ./kid_shogi --num-tries 1000

Engine protocol (--engine mode):
  - Each engine runs as a persistent subprocess per game.
  - The match runner sends the initial FEN to the Sente engine.
  - Each engine reads a FEN, makes its move, and prints either:
      • The new FEN (game continues — fed directly to the other engine), or
      • A result string: "1-0" (Sente wins), "0-1" (Gote wins), "1/2-1/2" (draw).

Game alternation (per pair):
  - Engine A plays Sente in odd-numbered games of the pair, Gote in even ones.
"""

import subprocess
import sys
import itertools
import time

sys.stdout.reconfigure(line_buffering=True)

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


def play_game(cmd_a: list[str], cmd_b: list[str], a_is_sente: bool) -> tuple[str, float, float]:
    """Play one game. Returns (result, time_a_secs, time_b_secs)."""
    eng_a = start_engine(cmd_a)
    eng_b = start_engine(cmd_b)

    if a_is_sente:
        sente_eng, gote_eng = eng_a, eng_b
        sente_label, gote_label = "A", "B"
    else:
        sente_eng, gote_eng = eng_b, eng_a
        sente_label, gote_label = "B", "A"

    result = "draw"
    # time_by_label["A"] / ["B"] = total seconds spent waiting for that engine
    time_by_label: dict[str, float] = {"A": 0.0, "B": 0.0}
    try:
        send_line(sente_eng, INITIAL_FEN)
        movers = [(sente_eng, sente_label), (gote_eng, gote_label)]
        mover_idx = 0
        while True:
            current_eng, current_label = movers[mover_idx]
            t0 = time.perf_counter()
            response = recv_line(current_eng)
            time_by_label[current_label] += time.perf_counter() - t0
            if response in RESULTS:
                if response == "1/2-1/2":
                    result = "draw"
                elif response == "1-0":
                    result = sente_label
                else:
                    result = gote_label
                break
            mover_idx = 1 - mover_idx
            send_line(movers[mover_idx][0], response)
    finally:
        for eng in (eng_a, eng_b):
            try:
                eng.stdin.close()
                eng.wait(timeout=5)
            except Exception:
                eng.kill()

    return result, time_by_label["A"], time_by_label["B"]


def run_match(games: int, cmd_a: list[str], cmd_b: list[str],
              label_a: str, label_b: str, jobs: int = 1) -> tuple[int, int, int, float, float]:
    """Run `games` games. Returns (wins_a, draws, wins_b, total_time_a, total_time_b)."""
    from concurrent.futures import ThreadPoolExecutor, as_completed

    def run_one(i):
        a_is_sente = (i % 2 == 0)
        result, ta, tb = play_game(cmd_a, cmd_b, a_is_sente)
        return i, a_is_sente, result, ta, tb

    wins_a = draws = wins_b = 0
    total_time_a = total_time_b = 0.0
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {pool.submit(run_one, i): i for i in range(games)}
        for fut in as_completed(futures):
            try:
                i, a_is_sente, result, ta, tb = fut.result()
            except RuntimeError as e:
                print(f"  Game {futures[fut]+1}: ERROR — {e}")
                continue

            total_time_a += ta
            total_time_b += tb

            if result == "draw":
                draws += 1
                tag = "draw"
            elif result == "A":
                wins_a += 1
                tag = f"{label_a} wins"
            else:
                wins_b += 1
                tag = f"{label_b} wins"

            color_a = "Sente" if a_is_sente else "Gote "
            print(f"  Game {i+1:3d} ({label_a}={color_a}): {tag}"
                  f"  [{label_a} {ta:.1f}s, {label_b} {tb:.1f}s]", flush=True)

    return wins_a, draws, wins_b, total_time_a, total_time_b


def parse_args(argv):
    # Extract -j/--jobs before splitting on --
    jobs = 1
    filtered = []
    i = 0
    while i < len(argv):
        if argv[i] in ('-j', '--jobs') and i + 1 < len(argv):
            jobs = int(argv[i + 1])
            i += 2
        elif argv[i].startswith('--jobs='):
            jobs = int(argv[i].split('=', 1)[1])
            i += 1
        else:
            filtered.append(argv[i])
            i += 1
    argv = filtered

    if len(argv) < 4 or "--" not in argv:
        print(__doc__)
        sys.exit(1)
    games = int(argv[0])
    engines = []
    current = []
    for tok in argv[1:]:
        if tok == "--":
            if current:
                engines.append(current)
            current = []
        else:
            current.append(tok)
    if current:
        engines.append(current)
    if len(engines) < 2:
        print("Need at least two engine specs separated by '--'")
        sys.exit(1)
    return games, engines, jobs


def engine_label(cmd: list[str], idx: int) -> str:
    """Short label for display: index + last component of binary path."""
    import os
    return f"E{idx+1}({os.path.basename(cmd[0])})"


def main():
    games, engines, jobs = parse_args(sys.argv[1:])
    labels = [engine_label(cmd, i) for i, cmd in enumerate(engines)]
    if jobs > 1:
        print(f"Running up to {jobs} games in parallel.")

    if len(engines) == 2:
        # Simple two-engine match
        w, d, l, ta, tb = run_match(games, engines[0], engines[1], labels[0], labels[1], jobs)
        print()
        print(f"Results after {games} games ({labels[0]} vs {labels[1]}):")
        print(f"  {labels[0]} wins : {w}")
        print(f"  Draws        : {d}")
        print(f"  {labels[1]} wins : {l}")
        print(f"  Think time   : {labels[0]} {ta:.1f}s total ({ta/max(w+d+l,1):.2f}s/game)"
              f", {labels[1]} {tb:.1f}s total ({tb/max(w+d+l,1):.2f}s/game)")
        return

    # Round-robin: every pair plays `games` games
    n = len(engines)
    # scores[i] = points (win=1, draw=0.5, loss=0)
    scores = [0.0] * n
    wins   = [[0] * n for _ in range(n)]
    draws  = [[0] * n for _ in range(n)]
    total_time = [0.0] * n  # cumulative think time per engine index

    pairs = list(itertools.combinations(range(n), 2))
    total_pairs = len(pairs)
    for match_num, (i, j) in enumerate(pairs, 1):
        print(f"\n=== Match {match_num}/{total_pairs}: {labels[i]} vs {labels[j]} ===")
        w, d, l, ta, tb = run_match(games, engines[i], engines[j], labels[i], labels[j], jobs)
        wins[i][j] = w
        wins[j][i] = l
        draws[i][j] = draws[j][i] = d
        scores[i] += w + 0.5 * d
        scores[j] += l + 0.5 * d
        total_time[i] += ta
        total_time[j] += tb

    # Final standings
    print("\n" + "=" * 60)
    print("FINAL STANDINGS")
    print("=" * 60)
    order = sorted(range(n), key=lambda x: -scores[x])
    col = max(len(lb) for lb in labels) + 2
    header = (f"{'Engine':<{col}}"
              + "".join(f"{labels[j]:>6}" for j in range(n))
              + f"{'Points':>8}  {'Think(s)':>10}")
    print(header)
    print("-" * len(header))
    for i in order:
        row = f"{labels[i]:<{col}}"
        for j in range(n):
            if i == j:
                row += f"{'---':>6}"
            else:
                row += f"{wins[i][j]}+{draws[i][j]:>3}"
        row += f"{scores[i]:>8.1f}  {total_time[i]:>10.1f}"
        print(row)


if __name__ == "__main__":
    main()
