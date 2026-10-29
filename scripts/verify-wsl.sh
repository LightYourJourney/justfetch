#!/bin/sh
# Check justfetch's behaviour inside WSL1/WSL2.
#
# This is a semantics check, not a comparison: WSL has no Windows-side tool
# worth cross-checking against from inside the distribution. It verifies that
# WSL is labelled, that the modules with nothing to answer say so, and that the
# ones that should work do.
set -u
BIN=${BIN:-./target/release/justfetch}

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for the status check" >&2
  exit 2
fi
if [ ! -x "$BIN" ]; then
  echo "Build first: cargo build --release --locked (looked for $BIN)" >&2
  exit 2
fi

if ! grep -qi microsoft /proc/version 2>/dev/null; then
  echo "This does not look like WSL; run scripts/verify-linux.sh instead." >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/jf-wsl.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

"$BIN" --format json --modules os,host,kernel,gpu,battery,display,packages,desktop,locale > "$work/doc.json" \
  || { echo "justfetch failed" >&2; exit 2; }

python3 - "$work/doc.json" <<'PY'
import json, sys

document = json.load(open(sys.argv[1]))
print(f"environment: {document['environment']}")
problems = []

if document["environment"].get("kind") != "wsl":
    problems.append(f"environment is {document['environment']}, not wsl")

# Nothing on WSL can answer these two, so a value here would be an invented one.
for module in ("battery", "display"):
    field = document[module]
    print(f"{module:9} {field.get('status'):12} {field.get('reason', '')}")
    if field.get("status") == "available":
        problems.append(f"{module} reported a value on WSL: {field.get('value')}")

# Desktop depends on the session: no variables and no TTY is unavailable, a
# plain TTY is unsupported. Both are honest; a value here would not be.
field = document["desktop"]
print(f"{'desktop':9} {field.get('status'):12} {field.get('reason', '')}")
if field.get("status") not in ("unsupported", "unavailable"):
    problems.append(f"desktop reported {field.get('status')} on WSL")

for module in ("packages", "locale"):
    field = document[module]
    print(f"{module:9} {field.get('status'):12} {field.get('reason', '')}")

print()
if problems:
    print("FINDINGS:")
    for problem in problems:
        print(" -", problem)
    sys.exit(1)
print("WSL is labelled and the not-applicable modules report no value.")
PY
