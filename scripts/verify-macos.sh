#!/bin/sh
# Compare justfetch's macOS collectors with the system's own tools.
#
# Read-only and repeatable. Prints one line per module with a verdict:
# match, mismatch, uncomparable or unsupported. Exit 1 on any mismatch.
set -u
here=$(cd -- "$(dirname -- "$0")" && pwd)
BIN=${BIN:-./target/release/justfetch}

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for the comparison" >&2
  exit 2
fi
if ! python3 -c 'import json' >/dev/null 2>&1; then
  echo "python3 is on PATH but does not run; check the interpreter" >&2
  exit 2
fi
if [ ! -x "$BIN" ]; then
  echo "Build first: cargo build --release --locked (looked for $BIN)" >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/jf-macos.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

"$BIN" --format json --modules gpu,battery,display,packages,desktop,locale > "$work/doc.json" \
  || { echo "justfetch failed" >&2; exit 2; }

# Native observations. Only non-identifying values are recorded: no serial
# numbers, no user or host names.
python3 - "$work" <<'PY' || { echo "failed to gather native observations" >&2; exit 2; }
import json, re, subprocess, sys, pathlib

work = pathlib.Path(sys.argv[1])
values, tools = {}, {}

# Battery: pmset reports "87%; discharging; ...".
out = subprocess.run(["pmset", "-g", "batt"], capture_output=True, text=True).stdout
match = re.search(r"(\d+)%;\s*(\w+)", out)
if match:
    values["battery"] = {"percent": int(match.group(1)),
                         "state": {"charged": "full", "charging": "charging",
                                   "discharging": "discharging"}.get(match.group(2), "unknown")}
    tools["battery"] = "pmset"

# GPU: the chipset model line.
out = subprocess.run(["system_profiler", "SPDisplaysDataType"],
                     capture_output=True, text=True).stdout
names = re.findall(r"Chipset Model: (.+)", out)
if names:
    values["gpu"] = {"names": [n.strip() for n in names]}
    tools["gpu"] = "system_profiler"

# Packages: Homebrew's own list.
import shutil

def count(args):
    if not shutil.which(args[0]):
        return None
    result = subprocess.run(args, capture_output=True, text=True)
    return len([line for line in result.stdout.splitlines() if line.strip()]) if result.returncode == 0 else None

formulae = count(["brew", "list", "--formula"])
casks = count(["brew", "list", "--cask"])
if formulae is not None:
    values["packages"] = {"homebrew formulae": formulae}
    if casks is not None:
        values["packages"]["homebrew casks"] = casks
    tools["packages"] = "brew list"

# Locale: the process locale the shell exported.
import os
if os.environ.get("LANG"):
    values["locale"] = os.environ["LANG"]
    tools["locale"] = "$LANG"

# Display is deliberately absent: justfetch reports the current mode's logical
# points while system_profiler reports the panel, so they are different
# quantities and comparing them would manufacture a mismatch.
# Desktop is absent because macOS has no desktop environment in this sense.

(work / "native.json").write_text(json.dumps({"tools": tools, "values": values}, indent=2))
PY

python3 "$here/compare.py" --strict --justfetch "$work/doc.json" --native "$work/native.json"
