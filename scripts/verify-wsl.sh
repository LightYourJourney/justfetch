#!/bin/sh
# Check justfetch's behaviour inside WSL1/WSL2.
#
# The point is not that Linux detection works (verify-linux.sh covers that) but
# that WSL is labelled and that things WSL cannot answer are reported as such
# rather than guessed.
set -u

BIN=${BIN:-./target/release/justfetch}
if [ ! -x "$BIN" ]; then
  echo "Build first: cargo build --release --locked (looked for $BIN)" >&2
  exit 1
fi

printf '\n=== environment ===\n'
echo "proc version: $(head -1 /proc/version)"
echo "WSL_DISTRO_NAME=${WSL_DISTRO_NAME:-unset}"
echo "WSL_INTEROP=${WSL_INTEROP:-unset}"
if ! grep -qi microsoft /proc/version 2>/dev/null; then
  echo "This does not look like WSL; run scripts/verify-linux.sh instead." >&2
fi

printf '\n=== justfetch --format json ===\n'
"$BIN" --format json > /tmp/jf-wsl.json || { echo "justfetch failed" >&2; exit 1; }
python3 - <<'PY'
import json, sys
d = json.load(open('/tmp/jf-wsl.json'))
print('environment:', d['environment'])
expect = {'battery': {'unsupported', 'unavailable'},
          'display': {'unsupported', 'unavailable'},
          'desktop': {'unsupported', 'unavailable'}}
problems = []
for k in ('os','host','kernel','gpu','battery','display','packages','desktop','locale'):
    v = d.get(k, {})
    print(f"{k:9} {v.get('status'):12} {v.get('reason') or ''}")
    if k in expect and v.get('status') not in expect[k]:
        problems.append(f"{k} reported {v.get('status')}: {v.get('value')}")
if d['environment'].get('kind') != 'wsl':
    problems.append(f"environment is {d['environment']}, not wsl")
if problems:
    print("\nFINDINGS:")
    for p in problems: print(" -", p)
    sys.exit(1)
print("\nWSL labelling and the not-applicable states are as expected.")
PY
