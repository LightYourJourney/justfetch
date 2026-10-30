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
import json, os, pathlib, shutil, subprocess, sys

work = pathlib.Path(sys.argv[1])
values, tools, absent, not_applicable = {}, {}, [], []

# WSL has no battery or display of its own: those belong to the Windows host and
# no Linux interface exposes them. That is what `absent` records.
absent.extend(["battery", "display"])

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
