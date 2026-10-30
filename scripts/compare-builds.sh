#!/bin/sh
# Compare this tree's build with a published archive, field by field.
#
# Used to show that a change made after a release did not alter behaviour. It
# prints which fields differ and needs no network: point it at an archive you
# already downloaded.
#
#   scripts/compare-builds.sh /path/to/justfetch-vX-aarch64-apple-darwin.tar.gz
#
# Exit 0 when every non-volatile field is identical, 1 when one differs.
set -u
archive=${1:-}
if [ -z "$archive" ] || [ ! -f "$archive" ]; then
  echo "usage: $0 <published-archive.tar.gz>" >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/jf-compare.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

tar -xzf "$archive" -C "$work" || { echo "cannot extract $archive" >&2; exit 2; }
released=$(find "$work" -type f -name justfetch | head -1)
[ -x "$released" ] || { echo "no binary in $archive" >&2; exit 2; }

./target/release/justfetch --format json > "$work/current.json" || exit 2
"$released" --format json > "$work/released.json" || exit 2

/usr/bin/python3 - "$work/current.json" "$work/released.json" <<'PY'
import json, sys

current = json.load(open(sys.argv[1]))
released = json.load(open(sys.argv[2]))

# Fields that legitimately change between two runs of the same binary on the
# same machine. Everything else must be identical, `cpu` included: a refactor
# that changed a collector would show up in one of these.
volatile = {"uptime", "memory", "swap", "battery", "disk"}

differences = [
    key for key in current
    if key not in volatile and current.get(key) != released.get(key)
]
print("compared:", ", ".join(sorted(k for k in current if k not in volatile)))
print("excluded as volatile:", ", ".join(sorted(volatile)))
if differences:
    print("DIFFERENT:", ", ".join(differences))
    sys.exit(1)
print("identical on every non-volatile field")
PY
