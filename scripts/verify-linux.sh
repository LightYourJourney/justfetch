#!/bin/sh
# Compare justfetch's Linux collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# uncomparable or unsupported. Exit 1 on any mismatch, 2 on a setup problem.
#
# Only non-identifying values are read: counts, percentages, adapter names,
# resolutions and locales. No serial numbers, IP addresses or user names.
set -u
here=$(cd -- "$(dirname -- "$0")" && pwd)
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

python3 - "$work" <<'PY' || { echo "failed to gather native observations" >&2; exit 2; }
import glob, json, os, pathlib, re, shutil, subprocess, sys

work = pathlib.Path(sys.argv[1])
values, tools = {}, {}

def run(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else ""

# Battery: the first system battery in sysfs.
for bat in sorted(glob.glob("/sys/class/power_supply/BAT*")):
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

# Packages: each manager's own count, under the names justfetch reports, and
# only when the count is non-zero — that is when the collector reports it.
managers, manager_tools = {}, []
if shutil.which("dpkg-query"):
    count = len([l for l in run(["dpkg-query", "-W", "-f=${db:Status-Abbrev}\\n"]).splitlines()
                 if l.startswith("ii")])
    if count:
        managers["dpkg"] = count
        manager_tools.append("dpkg-query")
if shutil.which("pacman"):
    count = len([l for l in run(["pacman", "-Q"]).splitlines() if l.strip()])
    if count:
        managers["pacman"] = count
        manager_tools.append("pacman")
if managers:
    values["packages"] = managers
    tools["packages"] = ", ".join(manager_tools)

# GPU: PCI ids are exact, so compare those when the platform shows them; the
# name is for the human reading the table.
names, ids = [], []
for path in glob.glob("/proc/driver/nvidia/gpus/*/information"):
    for line in open(path, errors="replace"):
        if line.startswith("Model:"):
            names.append(line.split(":", 1)[1].strip())
if names:
    values["gpu"] = {"names": names}
    tools["gpu"] = "/proc/driver/nvidia"
elif shutil.which("lspci"):
    for line in run(["lspci", "-nn"]).splitlines():
        if not re.search(r"(VGA compatible|3D controller|Display controller)", line):
            continue
        token = re.search(r"\[([0-9a-fA-F]{4}:[0-9a-fA-F]{4})\]", line)
        if token:
            ids.append(token.group(1))
        names.append(line.split(": ", 1)[-1].strip())
    if ids:
        values["gpu"] = {"ids": ids, "names": names}
        tools["gpu"] = "lspci -nn"

# Display: sysfs knows the preferred mode, which is what Linux reports. The
# current mode lives in the compositor, so comparing against xrandr would
# manufacture a mismatch whenever the user is not running the preferred mode.
resolutions = []
for connector in sorted(glob.glob("/sys/class/drm/card*-*")):
    try:
        if open(f"{connector}/status").read().strip() != "connected":
            continue
        modes = open(f"{connector}/modes").read().split()
    except OSError:
        continue
    if modes:
        resolutions.append(modes[0])
if resolutions:
    values["display"] = resolutions
    tools["display"] = "sysfs modes (same source as the estimate)"

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
