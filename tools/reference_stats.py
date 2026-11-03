#!/usr/bin/env python3
"""Independent reference for the phase-2 acceptance test.

Recounts the hero's VPIP, PFR, 3-bet, WTSD and W$SD straight from the
Winamax text files, without any of the Rust code, following the
definitions of docs/counters.md:

  vpip      call/bet/raise preflop (posts excluded)
  pfr       raise preflop
  3-bet     faced exactly one raise made by someone else / re-raised then
  saw flop  still in the hand when the flop is dealt
  wtsd      saw flop and showed down ("shows" in the SHOW DOWN section)
  w$sd      went to showdown and net result > 0

Usage: python3 tools/reference_stats.py <fixture dirs...> > reference.json
"""
import json, os, re, sys

def amount(s):
    s = s.strip()
    if s.endswith('€'):
        e, _, c = s[:-1].partition('.')
        return int(e) * 100 + int((c + '00')[:2])
    return int(s)

def hands_in(text):
    parts = re.split(r'(?m)^(?=Winamax Poker - (?!Tournament summary))', text)
    return [p for p in parts if p.startswith('Winamax Poker - ') and '*** SUMMARY ***' in p]

def analyse(hand):
    lines = [l.rstrip('\r') for l in hand.split('\n')]
    seats = []
    for l in lines:
        m = re.match(r'^Seat \d+: (.+) \(([0-9.]+€?)(?:, [0-9.]+€ bounty)?\)$', l)
        if m and '***' not in l:
            seats.append(m.group(1))
        if l.startswith('*** ANTE/BLINDS'):
            break
    hero = None
    for l in lines:
        m = re.match(r'^Dealt to (.+) \[', l)
        if m:
            hero = m.group(1)
    if hero is None:
        return None
    names = sorted(seats, key=len, reverse=True)
    def who(l):
        for n in names:
            if l.startswith(n + ' '):
                return n, l[len(n) + 1:]
        return None, None
    section = None
    raises = 0
    first_raiser = None
    r = dict(vpip=0, pfr=0, opp3=0, cnt3=0, flop=0, sd=0)
    folded_pre = False
    invested = 0
    street_in = {}
    collected = 0
    opp3_seen = False
    for l in lines:
        if l.startswith('*** '):
            title = l[4:].split(' ***')[0]
            section = title
            if title in ('FLOP', 'TURN', 'RIVER'):
                street_in = {}
                if title == 'FLOP' and not folded_pre:
                    r['flop'] = 1
            continue
        if section in (None, 'SUMMARY') or not l:
            continue
        n, rest = who(l)
        if n is None:
            continue
        rest = rest.replace(' and is all-in', '')
        add = 0
        if rest.startswith('posts '):
            a = amount(rest.split(' ')[-1] if 'out of position' not in rest else rest.split(' ')[-4])
            add = a
            if 'ante' not in rest:
                street_in[n] = street_in.get(n, 0) + a
        elif rest.startswith('calls '):
            add = amount(rest[6:])
            street_in[n] = street_in.get(n, 0) + add
        elif rest.startswith('bets '):
            add = amount(rest[5:])
            street_in[n] = street_in.get(n, 0) + add
        elif rest.startswith('raises '):
            to = amount(rest.split(' to ')[1])
            add = to - street_in.get(n, 0)
            street_in[n] = to
        elif rest.startswith('collected '):
            if n == hero:
                collected += amount(rest.split(' ')[1])
        elif rest.startswith('shows ') and section == 'SHOW DOWN' and n == hero:
            r['sd'] = 1
        if n == hero:
            invested += add
        if section == 'PRE-FLOP' and not rest.startswith('posts '):
            kind = rest.split(' ')[0]
            if n == hero:
                if raises == 1 and first_raiser != hero and not opp3_seen:
                    opp3_seen = True
                    r['opp3'] = 1
                    r['cnt3'] = 1 if kind == 'raises' else 0
                if kind in ('calls', 'raises', 'bets'):
                    r['vpip'] = 1
                if kind == 'raises':
                    r['pfr'] = 1
                if kind == 'folds':
                    folded_pre = True
            if kind == 'raises':
                raises += 1
                if first_raiser is None:
                    first_raiser = n
    net = collected - invested
    r['wtsd'] = 1 if (r['flop'] and r['sd']) else 0
    r['wsd'] = 1 if (r['sd'] and net > 0) else 0
    return r

def main(dirs):
    tot = dict(hands=0, vpip=0, pfr=0, opp3=0, cnt3=0, flop=0, wtsd=0, wsd=0, sd=0)
    for d in dirs:
        for root, _, files in os.walk(d):
            for f in sorted(files):
                if f.endswith('summary.txt'):
                    continue
                text = open(os.path.join(root, f), encoding='utf-8').read()
                for h in hands_in(text):
                    r = analyse(h)
                    if r is None:
                        continue
                    tot['hands'] += 1
                    for k in ('vpip', 'pfr', 'opp3', 'cnt3', 'flop', 'wtsd', 'wsd', 'sd'):
                        tot[k] += r[k]
    json.dump(tot, sys.stdout, indent=2)
    print()

if __name__ == '__main__':
    main(sys.argv[1:])
