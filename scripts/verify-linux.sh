#!/bin/sh
# Compare justfetch's Linux collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# unsupported, inconclusive, error or not covered. Runs compare.py in strict
# acceptance mode: exit 1 on a mismatch or error, 2 on a setup problem, 3 when a
# covered module had no comparison.
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
if ! python3 -c 'import json' >/dev/null 2>&1; then
  echo "python3 is on PATH but does not run; check the interpreter" >&2
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
values, tools, absent, not_applicable = {}, {}, [], []

def run(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else ""

# Battery: the first system battery in sysfs. The directory existing with no
# battery entry is what "this machine has no battery" looks like.
batteries = sorted(glob.glob("/sys/class/power_supply/BAT*"))
if not batteries and os.path.isdir("/sys/class/power_supply"):
    # The directory is readable and lists no battery: that is the evidence.
    absent.append("battery")
for bat in batteries:
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
    listing = subprocess.run(["lspci", "-nn"], capture_output=True, text=True)
    for line in (listing.stdout if listing.returncode == 0 else "").splitlines():
        if not re.search(r"(VGA compatible|3D controller|Display controller)", line):
            continue
        token = re.search(r"\[([0-9a-fA-F]{4}:[0-9a-fA-F]{4})\]", line)
        if token:
            ids.append(token.group(1))
        names.append(line.split(": ", 1)[-1].strip())
    if ids:
        values["gpu"] = {"ids": ids, "names": names}
        tools["gpu"] = "lspci -nn"
    elif (shutil.which("lspci") and listing.returncode == 0 and not names
          and not glob.glob("/sys/class/drm/card[0-9]*")):
        # lspci ran, listed no display adapter, and sysfs has no DRM card: the
        # machine really has none, which is what justfetch's "unsupported" claims.
        absent.append("gpu")

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

# No connected connector is evidence that nothing is attached, but only when
# sysfs was readable: `/sys/class/drm` exists but exposes no card at all.
if not resolutions and os.path.isdir("/sys/class/drm") and not glob.glob("/sys/class/drm/card[0-9]*"):
    absent.append("display")

# Desktop and locale: what the session actually exported.
desktop = os.environ.get("XDG_CURRENT_DESKTOP", "").split(":")[-1]
if desktop:
    values["desktop"] = desktop
    tools["desktop"] = "XDG_CURRENT_DESKTOP"
elif not os.environ.get("XDG_SESSION_DESKTOP") and not os.environ.get("DESKTOP_SESSION"):
    # No session variables at all: a headless session or a plain TTY. justfetch
    # reports unavailable or unsupported there, and this is the evidence.
    absent.append("desktop")
for key in ("LC_ALL", "LC_MESSAGES", "LANG"):
    if os.environ.get(key):
        values["locale"] = os.environ[key]
        tools["locale"] = key
        break

(work / "native.json").write_text(json.dumps(
    {"tools": tools, "values": values, "absent": absent,
     "not_applicable": not_applicable}, indent=2))
PY

python3 "$here/compare.py" --strict --justfetch "$work/doc.json" --native "$work/native.json"
