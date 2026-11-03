#!/usr/bin/env bash
# Exports every fixture hand as JSON for the front-end replayer test
# (app/src/test/fixtureHands.json, git-ignored). Run before `pnpm test`.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q -p lps-hh-parsers --example dump_json
out=app/src/test/fixtureHands.json
PY=$(command -v python3 || command -v python)
"$PY" - "$out" <<'PY'
import glob, json, subprocess, sys
import os
exe = 'target/debug/examples/dump_json' + ('.exe' if os.name == 'nt' else '')
hands = []
for f in sorted(glob.glob('crates/hh-parsers/fixtures/winamax/**/*.txt', recursive=True)):
    if f.endswith('summary.txt'):
        continue
    hands += json.loads(subprocess.run([exe, f], check=True, capture_output=True, text=True, encoding='utf-8').stdout)
json.dump(hands, open(sys.argv[1], 'w', encoding='utf-8'), separators=(',', ':'))
print(f'{len(hands)} hands -> {sys.argv[1]}')
PY
