"""Reference equities for the Equity Lab acceptance test, computed with eval7
(open-source hand evaluator and range parser, https://pypi.org/project/eval7/).

Every case is enumerated **exhaustively** (every deal of combos, every board):
the values are exact. Ranges are parsed by eval7's own parser (its syntax:
"40%(KQo)"), so the test also checks our notation parser independently.

Usage: python tools/reference_equity.py   (needs `pip install eval7`; a few minutes)
Writes crates/equity/tests/reference_cases.json.
"""
import itertools
import json
from multiprocessing import Pool
from pathlib import Path

import eval7

# (name, our notation per player, eval7 notation per player, board)
CASES = [
    ("AA vs KK", ["AsAh", "KsKh"], ["AsAh", "KsKh"], ""),
    ("AKs vs 22", ["AcKc", "2d2h"], ["AcKc", "2d2h"], ""),
    ("AKo vs QQ", ["AhKd", "QsQc"], ["AhKd", "QsQc"], ""),
    ("72o vs AA", ["7c2d", "AsAd"], ["7c2d", "AsAd"], ""),
    ("JTs vs AKo", ["JhTh", "AsKd"], ["JhTh", "AsKd"], ""),
    ("AKs vs AQs same suit", ["AsKs", "AcQc"], ["AsKs", "AcQc"], ""),
    ("54s vs AA", ["5s4s", "AhAd"], ["5s4s", "AhAd"], ""),
    ("KQs vs ATo", ["KhQh", "AcTd"], ["KhQh", "AcTd"], ""),
    ("AA vs KK vs QQ", ["AsAh", "KsKh", "QsQh"], ["AsAh", "KsKh", "QsQh"], ""),
    ("AKo vs JTs vs 88", ["AcKd", "JhTh", "8s8d"], ["AcKd", "JhTh", "8s8d"], ""),
    ("4-way", ["AhAd", "KcKs", "9h8h", "6c6d"], ["AhAd", "KcKs", "9h8h", "6c6d"], ""),
    ("flush draw vs set", ["AhKh", "QsQd"], ["AhKh", "QsQd"], "Qh7h2c"),
    ("OESD vs top pair", ["JcTc", "Ad9d"], ["JcTc", "Ad9d"], "9c8d2h"),
    ("turn: draw vs overpair", ["6s5s", "8c8d"], ["6s5s", "8c8d"], "7d4c2hKh"),
    ("hand vs range preflop", ["AhAd", "QQ+,AKs"], ["AhAd", "QQ+, AKs"], ""),
    ("range vs range preflop", ["AKs", "TT,JJ"], ["AKs", "TT, JJ"], ""),
    ("weighted range preflop", ["KQo:40%,AJs", "88-66"], ["40%(KQo), AJs", "88-66"], ""),
    ("range vs random, flop", ["AA,KK,AK", "random"],
     ["AA, KK, AK", "22+, A2+, K2+, Q2+, J2+, T2+, 92+, 82+, 72+, 62+, 52+, 42+, 32"], "Ah7c2d"),
    ("3 ranges, turn", ["AA,KK", "QQ,JJ", "AK"], ["AA, KK", "QQ, JJ", "AK"], "Ks9h4d2c"),
    ("weighted ranges, river", ["TT+,AQ:50%", "JJ,99,KQs"], ["TT+, 50%(AQ)", "JJ, 99, KQs"], "Qs9d4c2h7s"),
]

DECK = [eval7.Card(r + s) for r in "23456789TJQKA" for s in "cdhs"]


def deals(ranges, board):
    """Every (weight, hands) with no shared card."""
    used0 = {str(c) for c in board}

    def rec(p, used, w, hands):
        if p == len(ranges):
            yield w, list(hands)
            return
        for hand, wh in ranges[p]:
            names = {str(c) for c in hand}
            if names & used or wh == 0:
                continue
            yield from rec(p + 1, used | names, w * wh, hands + [list(hand)])

    yield from rec(0, used0, 1.0, [])


def one_deal(args):
    w, hands, board = args  # card names (eval7 cards cannot be sent to worker processes)
    hands = [[eval7.Card(c) for c in h] for h in hands]
    board = [eval7.Card(c) for c in board]
    used = {str(c) for h in hands for c in h} | {str(c) for c in board}
    rest = [c for c in DECK if str(c) not in used]
    n = len(hands)
    eq = [0.0] * n
    boards = 0
    for extra in itertools.combinations(rest, 5 - len(board)):
        full = board + list(extra)
        vals = [eval7.evaluate(h + full) for h in hands]
        best = max(vals)
        winners = [i for i, v in enumerate(vals) if v == best]
        for i in winners:
            eq[i] += 1.0 / len(winners)
        boards += 1
    return w, [e / boards for e in eq]


def main():
    out = []
    with Pool() as pool:
        for name, ours, theirs, board in CASES:
            b = [eval7.Card(board[i:i + 2]) for i in range(0, len(board), 2)]
            ranges = [eval7.HandRange(r).hands for r in theirs]
            jobs = [(w, [[str(c) for c in h] for h in hs], [str(c) for c in b]) for w, hs in deals(ranges, b)]
            total = 0.0
            eq = [0.0] * len(ours)
            for w, e in pool.imap_unordered(one_deal, jobs, chunksize=1):
                total += w
                for i, x in enumerate(e):
                    eq[i] += w * x
            eq = [x / total for x in eq]
            print(f"{name}: {', '.join(f'{x * 100:.3f} %' for x in eq)}", flush=True)
            out.append({"name": name, "players": ours, "eval7": theirs, "board": board, "equity": [round(x, 6) for x in eq]})
    path = Path(__file__).resolve().parents[1] / "crates/equity/tests/reference_cases.json"
    path.write_text(json.dumps(out, indent=2) + "\n")
    print("wrote", path)


if __name__ == "__main__":
    main()
