#!/bin/sh
# Check justfetch's behaviour inside WSL1/WSL2.
#
# Two parts: the comparison the Linux script does, in strict acceptance mode, for
# the modules WSL can answer (packages, locale), and a check that WSL is
# labelled and that the modules with nothing to answer report no value.
# Exit status follows compare.py: 1 mismatch or error, 2 setup problem, 3 a
# covered module had no comparison; a labelling failure is 1.
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
if ! grep -qi microsoft /proc/version 2>/dev/null; then
  echo "This does not look like WSL; run scripts/verify-linux.sh instead." >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/jf-wsl.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

"$BIN" --format json --modules os,host,kernel,gpu,battery,display,packages,desktop,locale > "$work/doc.json" \
  || { echo "justfetch failed" >&2; exit 2; }

python3 - "$work" <<'PY' || { echo "failed to gather native observations" >&2; exit 2; }
import glob, json, os, pathlib, shutil, subprocess, sys

work = pathlib.Path(sys.argv[1])
values, tools, absent, not_applicable = {}, {}, [], []

# Every absence is a look that came back empty, not an assumption about WSL.
# WSL2 can pass a GPU through (`/dev/dxg`), so the GPU check is its own.
def read_dir(path):
    """Entries, [] when the path does not exist, None when it could not be read.

    The difference matters: a WSL guest may have no /sys/class/drm at all, which
    is evidence that nothing is there, while EACCES is evidence of nothing.
    """
    try:
        return os.listdir(path)
    except FileNotFoundError:
        return []
    except OSError:
        return None

supplies = read_dir("/sys/class/power_supply")
if supplies is not None:
    present = False
    for name in supplies:
        try:
            if open(f"/sys/class/power_supply/{name}/type").read().strip() == "Battery":
                present = True
        except OSError:
            supplies = None
            break
    if supplies is not None and not present:
        absent.append("battery")

drm = read_dir("/sys/class/drm")
if drm is not None:
    connected = False
    for name in drm:
        if "-" not in name:
            continue
        try:
            if open(f"/sys/class/drm/{name}/status").read().strip() == "connected":
                connected = True
        except OSError:
            drm = None
            break
    if drm is not None and not connected:
        absent.append("display")
    # justfetch reads /sys/class/drm only, so "no DRM card in a readable
    # directory" is exactly the evidence for its `unsupported`. A Windows GPU
    # passed through as /dev/dxg is not something justfetch claims to see, and
    # the verifier does not judge the machine's hardware — only the claim.
    has_card = any(n.startswith("card") and n[4:].isdigit() for n in drm or [])
    if drm is not None and not has_card:
        absent.append("gpu")

# No desktop-session variables: WSLg may set WAYLAND_DISPLAY for applications,
# but justfetch's desktop module reads the desktop variables, so match that.
if not any(os.environ.get(name) for name in
           ("XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION")):
    absent.append("desktop")

if shutil.which("dpkg-query"):
    result = subprocess.run(["dpkg-query", "-W", "-f=${db:Status-Abbrev}\\n"],
                            capture_output=True, text=True)
    count = len([l for l in result.stdout.splitlines() if l.startswith("ii")])
    if count:
        values["packages"] = {"dpkg": count}
        tools["packages"] = "dpkg-query"

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
compared=$?

python3 - "$work/doc.json" <<'PY'
import json, sys

document = json.load(open(sys.argv[1]))
print(f"environment: {document['environment']}")
problems = []

if document["environment"].get("kind") != "wsl":
    problems.append(f"environment is {document['environment']}, not wsl")

# Nothing on WSL can answer these, so a value would be an invented one.
for module in ("battery", "display"):
    field = document[module]
    if field.get("status") == "available":
        problems.append(f"{module} reported a value on WSL: {field.get('value')}")

# Desktop depends on the session: no variables and no TTY is unavailable, a
# plain TTY is unsupported. Both are honest; a value here would not be.
field = document["desktop"]
if field.get("status") not in ("unsupported", "unavailable"):
    problems.append(f"desktop reported {field.get('status')} on WSL")

if problems:
    print("FINDINGS:")
    for problem in problems:
        print(" -", problem)
    sys.exit(1)
print("WSL is labelled and the not-applicable modules report no value.")
PY
checked=$?

# Keep compare.py's codes distinct: collapsing 2 or 3 into 1 would hide whether
# the environment was wrong or the evidence was missing.
if [ "$compared" -ne 0 ]; then exit "$compared"; fi
if [ "$checked" -ne 0 ]; then exit 1; fi
exit 0
