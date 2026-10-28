#!/bin/sh
# Cross-check justfetch's Linux collectors against the system's own tools.
#
# Read-only. Run on a real Linux machine and paste the output into
# docs/validation.md under a dated heading. Nothing here changes state, and no
# tool is required: a missing one leaves its line marked "not available".
set -u

say() { printf '\n=== %s ===\n' "$1"; }
have() { command -v "$1" >/dev/null 2>&1; }

BIN=${BIN:-./target/release/justfetch}
if [ ! -x "$BIN" ]; then
  echo "Build first: cargo build --release --locked (looked for $BIN)" >&2
  exit 1
fi

say "environment"
echo "kernel:    $(uname -srm)"
echo "distro:    $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
echo "container: ${container:-none}"
if [ -e /proc/1/cgroup ] && grep -qiE 'docker|kubepods|lxc' /proc/1/cgroup 2>/dev/null; then
  echo "container: yes (/proc/1/cgroup)"
fi

say "justfetch --format json"
"$BIN" --format json --modules gpu,battery,display,packages,desktop,locale > /tmp/jf.json || { echo "justfetch failed" >&2; exit 1; }
python3 - <<'PY'
import json
d = json.load(open('/tmp/jf.json'))
for k in ('gpu','battery','display','packages','desktop','locale'):
    v = d.get(k, {})
    print(f"{k:9} {v.get('status'):12} {v.get('reason') or v.get('source') or ''}")
PY

say "battery (compare with justfetch)"
if ls /sys/class/power_supply/BAT* >/dev/null 2>&1; then
  for b in /sys/class/power_supply/BAT*; do
    echo "$(basename "$b"): capacity=$(cat "$b/capacity" 2>/dev/null) status=$(cat "$b/status" 2>/dev/null) present=$(cat "$b/present" 2>/dev/null)"
  done
elif have upower; then upower -i "$(upower -e | grep -m1 BAT)" 2>/dev/null | head -20
else
  echo "no battery interface (desktops and containers usually have none)"
fi

say "packages (compare with justfetch)"
have dpkg-query && echo "dpkg:   $(dpkg-query -W -f='${db:Status-Abbrev}\n' 2>/dev/null | grep -c '^ii')"
have pacman      && echo "pacman: $(pacman -Q 2>/dev/null | wc -l)"
have rpm         && echo "rpm:    $(rpm -qa 2>/dev/null | wc -l)"
have nix-store   && echo "nix store paths: $(nix-store -q --references /run/current-system 2>/dev/null | wc -l)"

say "display (compare with justfetch)"
if [ -n "${WAYLAND_DISPLAY:-}" ] && have wlr-randr; then wlr-randr --json 2>/dev/null | head -20; fi
if [ -n "${DISPLAY:-}" ] && have xrandr; then xrandr --current 2>/dev/null | head -8; fi
for c in /sys/class/drm/card*-*; do
  [ -e "$c/status" ] || continue
  echo "$(basename "$c"): $(cat "$c/status") preferred=$(head -1 "$c/modes" 2>/dev/null)"
done

say "gpu (compare with justfetch)"
have lspci && lspci -nn 2>/dev/null | grep -Ei 'vga|3d|display' || echo "lspci not available"
ls /proc/driver/nvidia/gpus/*/information 2>/dev/null | while read -r f; do
  echo "$f"; grep -m1 'Model:' "$f"
done

say "desktop and locale (compare with justfetch)"
echo "XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-unset}"
echo "XDG_SESSION_DESKTOP=${XDG_SESSION_DESKTOP:-unset}"
echo "DESKTOP_SESSION=${DESKTOP_SESSION:-unset}"
echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}"
echo "SWAYSOCK=${SWAYSOCK:-unset}"
echo "HYPRLAND_INSTANCE_SIGNATURE=${HYPRLAND_INSTANCE_SIGNATURE:-unset}"
have locale && locale 2>/dev/null | head -3
echo "system locale file: $(cat /etc/locale.conf 2>/dev/null || cat /etc/default/locale 2>/dev/null || echo 'none')"

say "paste into docs/validation.md"
echo "Record: date, distro and version, kernel, whether it was a VM/container,"
echo "the six statuses above, and any line where justfetch and the native tool"
echo "disagree. A disagreement is a finding, not something to smooth over."
