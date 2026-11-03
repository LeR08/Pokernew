"""Writes crates/evaluator/tests/eval7_pairs.txt: random pairs of 7-card hands and
which one wins according to eval7 (public open-source evaluator).

Usage: python tools/reference_evaluator.py   (needs `pip install eval7`)
Each line: "<hand A> <hand B> <1|0|-1>" (1 = A wins, 0 = split).
"""
import random
from pathlib import Path

import eval7

random.seed(20261103)
deck = [r + s for r in "23456789TJQKA" for s in "cdhs"]
out = []
for _ in range(20000):
    cards = random.sample(deck, 9 if random.random() < 0.5 else 14)
    if len(cards) == 9:  # shared board: realistic showdowns, many ties
        a, b = cards[:2] + cards[4:], cards[2:4] + cards[4:]
    else:
        a, b = cards[:7], cards[7:]
    va = eval7.evaluate([eval7.Card(c) for c in a])
    vb = eval7.evaluate([eval7.Card(c) for c in b])
    out.append(f"{''.join(a)} {''.join(b)} {(va > vb) - (va < vb)}")
Path(__file__).resolve().parents[1].joinpath("crates/evaluator/tests/eval7_pairs.txt").write_text("\n".join(out) + "\n")
print(len(out), "pairs")
