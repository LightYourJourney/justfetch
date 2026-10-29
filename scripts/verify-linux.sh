#!/bin/sh
# Compare justfetch's Linux collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# uncomparable or unsupported. Exit 1 on any mismatch, 2 on a setup problem.
#
# Only non-identifying values are read: counts, percentages, adapter names,
# resolutions and locales. No serial numbers, IP addresses or user names.
set -u
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN=${BIN:-./target/release/justfetch}

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for the comparison" >&2
  exit 2
fi
if [ ! -x "$BIN" ]; then
  echo "Build first: cargo build --release --locked (looked for $BIN)" >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/jf-linux.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

"$BIN" --format json --modules gpu,battery,display,packages,desktop,locale > "$work/doc.json" \
  || { echo "justfetch failed" >&2; exit 2; }

python3 - "$work" <<'PY'
import glob, json, os, pathlib, re, subprocess, sys

work = pathlib.Path(sys.argv[1])
values, tools = {}, {}

def run(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else ""

# Battery: the first system battery in sysfs.
bats = sorted(glob.glob("/sys/class/power_supply/BAT*"))
for bat in bats:
    try:
        scope = open(f"{bat}/scope").read().strip()
        if scope and scope != "System":
            continue
        capacity = int(open(f"{bat}/capacity").read().strip())
        status = open(f"{bat}/status").read().strip().lower()
    except (OSError, ValueError):
        continue
    values["battery"] = {"percent": capacity,
                         "state": {"charging": "charging", "discharging": "discharging",
                                   "full": "full"}.get(status, "unknown")}
    tools["battery"] = bat
    break

# Packages: each manager's own count, under the name justfetch uses.
if subprocess.run(["sh", "-c", "command -v dpkg-query"], capture_output=True).returncode == 0:
    count = len([l for l in run(["dpkg-query", "-W", "-f=${db:Status-Abbrev}\n"]).splitlines()
                 if l.startswith("ii")])
    values.setdefault("packages", {})["dpkg"] = count
    tools["packages"] = "dpkg-query"
if subprocess.run(["sh", "-c", "command -v pacman"], capture_output=True).returncode == 0:
    count = len([l for l in run(["pacman", "-Q"]).splitlines() if l.strip()])
    values.setdefault("packages", {})["pacman"] = count
    tools.setdefault("packages", "pacman")

# GPU: the adapter names the kernel exposes, when it exposes any.
names = []
for path in glob.glob("/proc/driver/nvidia/gpus/*/information"):
    for line in open(path, errors="replace"):
        if line.startswith("Model:"):
            names.append(line.split(":", 1)[1].strip())
if names:
    values["gpu"] = names
    tools["gpu"] = "/proc/driver/nvidia"

# Display: the current mode, which only the display server knows.
if os.environ.get("DISPLAY") and subprocess.run(["sh", "-c", "command -v xrandr"],
                                                capture_output=True).returncode == 0:
    current = re.search(r"^\s+(\d+x\d+).*\*", run(["xrandr", "--current"]), re.MULTILINE)
    if current:
        values["display"] = [current.group(1)]
        tools["display"] = "xrandr"

# Desktop and locale: what the session actually exported.
desktop = os.environ.get("XDG_CURRENT_DESKTOP", "").split(":")[-1]
if desktop:
    values["desktop"] = desktop
    tools["desktop"] = "XDG_CURRENT_DESKTOP"
for key in ("LC_ALL", "LC_MESSAGES", "LANG"):
    if os.environ.get(key):
        values["locale"] = os.environ[key]
        tools["locale"] = key
        break

(work / "native.json").write_text(json.dumps({"tools": tools, "values": values}, indent=2))
PY

python3 "$here/compare.py" --justfetch "$work/doc.json" --native "$work/native.json"
