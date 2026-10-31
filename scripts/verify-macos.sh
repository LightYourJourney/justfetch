#!/bin/sh
# Compare justfetch's macOS collectors with the system's own tools.
#
# Read-only and repeatable. Prints one line per module with a verdict: match,
# mismatch, unsupported, inconclusive, error or not covered. Runs compare.py in
# strict acceptance mode: exit 1 on a mismatch or error, 2 on a setup problem,
# 3 when a covered module had no comparison.
set -u
here=$(cd -- "$(dirname -- "$0")" && pwd)
BIN=${BIN:-$here/../target/release/justfetch}

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
import json, os, re, subprocess, sys, pathlib, shutil

work = pathlib.Path(sys.argv[1])
values, tools, absent, not_applicable = {}, {}, [], []

# macOS has no desktop environment or window manager in justfetch's sense. The
# independent check: none of the variables a desktop session sets are present.
session_vars = ("XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION",
                "SWAYSOCK", "HYPRLAND_INSTANCE_SIGNATURE", "WAYLAND_DISPLAY")
if not any(os.environ.get(name) for name in session_vars):
    absent.append("desktop")

# The battery check distinguishes "no battery" from "could not read". On a Mac
# with no battery pmset prints "No batteries available."
battery_out = subprocess.run(["pmset", "-g", "batt"], capture_output=True, text=True).stdout
if "no batter" in battery_out.lower():
    absent.append("battery")

# Battery: pmset reports "87%; discharging; ...".
match = re.search(r"(\d+)%;\s*([^;]+)", battery_out)
if match:
    # pmset's state phrases: "charged", "charging", "finishing charge",
    # "discharging", "AC attached". Split on whitespace and map the first word
    # that carries meaning, so "finishing charge" is charging, not "finishing".
    phrase = match.group(2).strip().lower()
    if phrase.startswith("finishing") or phrase.startswith("charging"):
        state = "charging"
    elif phrase.startswith("charged") or phrase.startswith("full"):
        state = "full"
    elif phrase.startswith("discharging"):
        state = "discharging"
    elif phrase.startswith("ac attached"):
        state = "unknown"  # plugged in, pmset reports no charge state
    else:
        state = None  # do not compare a state we cannot name
    values["battery"] = {"percent": int(match.group(1)), "state": state}
    if state is None:
        del values["battery"]["state"]
    tools["battery"] = "pmset"

# GPU: the chipset model line.
out = subprocess.run(["system_profiler", "SPDisplaysDataType"],
                     capture_output=True, text=True).stdout
names = re.findall(r"Chipset Model: (.+)", out)
if names:
    values["gpu"] = {"names": [n.strip() for n in names]}
    tools["gpu"] = "system_profiler"

# Packages: declared not comparable. justfetch counts directories in Cellar and
# Caskroom, which its own source says does not claim to equal `brew list`; with a
# formula installed at two versions the two answers differ by design, so a
# comparison would report a mismatch that is not one.

# Locale: the same precedence the detector uses, or a shell that sets LC_ALL
# would compare its value against LANG and report a mismatch.
for key in ("LC_ALL", "LC_MESSAGES", "LANG"):
    if os.environ.get(key):
        values["locale"] = os.environ[key]
        tools["locale"] = f"${key}"
        break

# Display is declared not comparable: justfetch reports the current mode's
# logical points while system_profiler reports the panel, so they are different
# quantities and a comparison would manufacture a mismatch.
not_applicable.extend(["display", "packages"])

(work / "native.json").write_text(json.dumps(
    {"tools": tools, "values": values, "absent": absent,
     "not_applicable": not_applicable}, indent=2))
PY

python3 "$here/compare.py" --strict --justfetch "$work/doc.json" --native "$work/native.json"
