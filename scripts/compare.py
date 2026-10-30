#!/usr/bin/env python3
"""Compare a justfetch document with observations from the platform's own tools.

Used by scripts/verify-*. The platform script writes both files:

  doc.json    the output of `justfetch --format json --modules ...`
  native.json {
                "tools":  {"battery": "pmset", ...},
                "values": {"battery": {"percent": 87, "state": "discharging"},
                           "packages": {"homebrew formulae": 883},
                           "gpu": {"names": ["Apple M3"]}, "display": ["1470x956"],
                           "desktop": "Sway", "locale": "zh_CN.UTF-8"}
              }

Each module gets one verdict:

  match         the two agree
  mismatch      they disagree — the interesting case
  unsupported   justfetch says the question does not apply, and the platform
                tool agrees there is nothing
  inconclusive  there is no comparison: one side has nothing
  error         justfetch reported a failure; always a finding
  not covered   the script gathered no observation, and justfetch did not say
                the question does not apply

Exit status: 0 pass, 1 mismatch or error, 2 bad input, 3 (strict only) a covered
module had no comparison. Only non-identifying values are printed: counts,
resolutions, names, locales.
"""

import argparse
import json
import sys
from collections import Counter

MODULES = ["gpu", "battery", "display", "packages", "desktop", "locale"]


def show(value, limit=34):
    """One line for the table, truncated at `limit` characters."""
    text = value if isinstance(value, str) else json.dumps(value, ensure_ascii=False)
    return text if len(text) <= limit else text[: limit - 1] + "…"


def battery_text(value):
    units = value.get("units", [])
    if not units:
        return ""
    first = units[0]
    percent = "?" if first.get("percent") is None else f"{first['percent']}%"
    return f"{percent} {first.get('state', '?')}"


def packages_text(value):
    return ", ".join(f"{m['name']} {m['count']}" for m in value.get("managers", []))


def gpu_text(value):
    return ", ".join(
        device.get("name") or f"{device.get('vendor', '?')} {device.get('id', '')}".strip()
        for device in value.get("devices", [])
    )


def display_text(value):
    return ", ".join(
        f"{unit['width']}x{unit['height']}" for unit in value.get("displays", [])
    )


def desktop_text(value):
    return value.get("name", "")


def locale_text(value):
    return value.get("process") or value.get("system") or ""


TEXTS = {
    "battery": battery_text,
    "packages": packages_text,
    "gpu": gpu_text,
    "display": display_text,
    "desktop": desktop_text,
    "locale": locale_text,
}


def compare(module, value, native):
    """Return True when the justfetch value agrees with the native observation."""
    if module == "battery":
        units = value.get("units") or [{}]
        return units[0].get("percent") == native.get("percent") and (
            native.get("state") is None or units[0].get("state") == native["state"]
        )
    if module == "packages":
        # Both directions: a manager or a count that only one side reports is a
        # disagreement, not a match on the subset they share.
        ours = {manager["name"]: manager["count"] for manager in value.get("managers", [])}
        return bool(native) and ours == native
    if module == "gpu":
        devices = value.get("devices") or []
        # Multisets, not sets: two identical adapters are two adapters, and a
        # document reporting one of them must not match a machine with two.
        if native.get("ids"):
            ours = Counter(
                str(d.get("id", "")).lower() for d in devices if d.get("id")
            )
            return bool(ours) and ours == Counter(str(i).lower() for i in native["ids"])
        ours = Counter(
            (device.get("name") or "").strip().lower() for device in devices
            if (device.get("name") or "").strip()
        )
        theirs = Counter(
            name.strip().lower() for name in native.get("names", []) if name.strip()
        )
        return bool(ours) and ours == theirs
    if module == "display":
        # Also multisets: two monitors at the same resolution are two monitors.
        ours = Counter(
            f"{unit['width']}x{unit['height']}" for unit in value.get("displays", [])
        )
        return bool(ours) and ours == Counter(str(resolution) for resolution in native)
    if module in ("desktop", "locale"):
        ours, theirs = TEXTS[module](value).lower(), str(native).lower()
        return bool(ours) and bool(theirs) and (theirs in ours or ours in theirs)
    return False


def no_observation(module, native):
    """True when the platform tool produced nothing worth comparing."""
    if native is None:
        return True
    if isinstance(native, (dict, list, str)) and len(native) == 0:
        return True
    # `{"names": [], "ids": []}` is a container that carries nothing.
    if module == "gpu" and isinstance(native, dict):
        return not (native.get("names") or native.get("ids"))
    # A battery reading without a percentage is not an observation either.
    return module == "battery" and isinstance(native, dict) and "percent" not in native


def evaluate(module, field, native, absent):
    """Return (justfetch text, native text, verdict).

    The verdicts are the ones the acceptance procedure uses:

      match         the two agree
      mismatch      they disagree
      unsupported   justfetch says not applicable, and the script checked that
                    it really is (`absent` lists the modules it verified absent)
      inconclusive  there is no comparison — either side has nothing
      error         justfetch reported a failure; always a finding

    `unsupported` without that independent check is `not covered`: justfetch
    saying "not applicable" on its own is a claim, not evidence. A module the
    script declared it cannot compare (`--not-applicable`) is reported as
    `not comparable` and does not count against the run.
    """
    status = field.get("status")
    value = field.get("value") or {}
    if no_observation(module, native):
        native = None

    if status == "error":
        return f"error ({field.get('code', '?')})", show(native) if native else "—", "error"

    if status == "available":
        text = TEXTS[module](value)
        if native is None:
            return text, "—", "inconclusive"
        return text, show(native), "match" if compare(module, value, native) else "mismatch"

    if status == "unsupported":
        if native is not None:
            # The platform has an answer justfetch declined to give.
            return status, show(native), "mismatch"
        if module in absent:
            # The script looked and found nothing; that is evidence.
            return status, "—", "unsupported"
        # Nothing was observed either way: the claim stands unverified.
        return status, "—", "inconclusive"
    # unavailable, redacted, deselected: no value on our side.
    return status, show(native) if native is not None else "—", "inconclusive"


def check_inputs(document, native):
    """Return a description of the first malformed input, or None.

    `evaluate` indexes into these structures, so anything it would trip over is
    rejected here as bad input (exit 2) rather than surfacing as a crash.
    """
    if not isinstance(document, dict):
        return "the justfetch document is not an object"
    for module in MODULES:
        field = document.get(module)
        if not isinstance(field, dict) or not isinstance(field.get("status"), str):
            return f"`{module}` is missing a status"
        if field["status"] != "available":
            continue
        value = field.get("value")
        problem = check_value(module, value)
        if problem:
            return problem
    if not isinstance(native, dict):
        return "the native file is not an object"
    if not isinstance(native.get("tools", {}), dict):
        return "native `tools` is not an object"
    for key in ("absent", "not_applicable"):
        names = native.get(key, [])
        if not isinstance(names, list) or not all(isinstance(m, str) for m in names):
            return f"native `{key}` must be a list of module names"
        unknown = [m for m in names if m not in MODULES]
        if unknown:
            return f"native `{key}` names unknown modules: {', '.join(unknown)}"
    values = native.get("values", {})
    if not isinstance(values, dict):
        return "native `values` is not an object"
    shapes = {
        "battery": dict,
        "packages": dict,
        "gpu": dict,
        "display": list,
        "desktop": str,
        "locale": str,
    }
    for module, expected in shapes.items():
        value = values.get(module)
        if value is not None and not isinstance(value, expected):
            return f"native `{module}` is {type(value).__name__}, expected {expected.__name__}"
    gpu = values.get("gpu")
    if isinstance(gpu, dict):
        for key in ("names", "ids"):
            if gpu.get(key) is not None and not (
                isinstance(gpu[key], list) and all(isinstance(v, str) for v in gpu[key])
            ):
                return f"native `gpu.{key}` must be a list of strings"
    if isinstance(values.get("packages"), dict) and not all(
        isinstance(count, int) for count in values["packages"].values()
    ):
        return "native `packages` must map names to integers"
    if isinstance(values.get("battery"), dict) and not isinstance(
        values["battery"].get("percent", 0), int
    ):
        return "native `battery.percent` must be an integer"
    return None


def check_value(module, value):
    if not isinstance(value, dict):
        return f"`{module}` says available without an object value"
    if module == "battery":
        units = value.get("units")
        if not isinstance(units, list) or not all(isinstance(u, dict) for u in units):
            return "`battery.units` must be a list of objects"
        for unit in units:
            if unit.get("percent") is not None and not isinstance(unit["percent"], int):
                return "`battery.units[].percent` must be an integer or null"
    if module == "packages":
        managers = value.get("managers")
        if not isinstance(managers, list) or not all(
            isinstance(m, dict) and isinstance(m.get("name"), str)
            and isinstance(m.get("count"), int)
            for m in managers
        ):
            return "`packages.managers` must be a list of {name, count}"
    if module == "gpu":
        devices = value.get("devices")
        if not isinstance(devices, list) or not all(isinstance(d, dict) for d in devices):
            return "`gpu.devices` must be a list of objects"
    if module == "display":
        displays = value.get("displays")
        if not isinstance(displays, list) or not all(
            isinstance(d, dict) and isinstance(d.get("width"), int)
            and isinstance(d.get("height"), int)
            for d in displays
        ):
            return "`display.displays` must be a list of {width, height}"
    if module == "desktop" and not isinstance(value.get("name"), str):
        return "`desktop.name` must be a string"
    if module == "locale":
        for key in ("process", "system"):
            if value.get(key) is not None and not isinstance(value[key], str):
                return f"`locale.{key}` must be a string or null"
    return None


def main():
    parser = argparse.ArgumentParser(
        description="Compare a justfetch document with native observations.")
    parser.add_argument("--justfetch", required=True)
    parser.add_argument("--native", required=True)
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Acceptance mode: a covered module with no comparison, or any error, "
             "fails the run (exit 3) instead of being reported and passing.",
    )
    args = parser.parse_args()

    try:
        # utf-8-sig accepts a BOM: Windows PowerShell 5.1 writes one.
        with open(args.justfetch, encoding="utf-8-sig") as handle:
            document = json.load(handle)
        with open(args.native, encoding="utf-8-sig") as handle:
            native = json.load(handle)
    except (OSError, ValueError) as error:
        print(f"cannot read input: {error}", file=sys.stderr)
        return 2

    malformed = check_inputs(document, native)
    if malformed:
        print(f"malformed input: {malformed}", file=sys.stderr)
        return 2

    tools = native.get("tools", {})
    values = native.get("values", {})
    absent = set(native.get("absent", []))
    not_applicable = set(native.get("not_applicable", []))

    # A module is covered when there is evidence behind its verdict: the
    # platform tool produced an observation, or the script checked and found
    # nothing to observe. justfetch saying "unsupported" is not evidence by
    # itself — that is what the `absent` list exists to supply.
    covered = {
        module
        for module in MODULES
        if not no_observation(module, values.get(module)) or module in absent
    }

    print(f"{'module':10} {'justfetch':34} {'platform tool':34} verdict")
    print("-" * 92)
    counts = {"match": 0, "mismatch": 0, "unsupported": 0, "inconclusive": 0, "error": 0}
    for module in MODULES:
        ours, theirs, result = evaluate(
            module, document[module], values.get(module), absent
        )
        if result == "inconclusive" and module not in covered:
            result = "not comparable" if module in not_applicable else "not covered"
        else:
            counts[result] += 1
        tool = tools.get(module)
        if tool and theirs != "—":
            theirs = f"{theirs} ({tool})"
        print(f"{module:10} {show(ours):34} {theirs:34} {result}")

    print("-" * 92)
    uncovered = [module for module in MODULES if module not in covered]
    gaps = [module for module in uncovered if module not in not_applicable]
    print(
        f"covered {len(covered)}/{len(MODULES)}: "
        f"{counts['match']} match, {counts['unsupported']} unsupported, "
        f"{counts['inconclusive']} inconclusive, {counts['mismatch']} mismatch, "
        f"{counts['error']} error"
        + (f"; declared not comparable: {', '.join(sorted(not_applicable))}"
           if not_applicable else "")
        + (f"; gap: {', '.join(gaps)}" if gaps else "")
    )

    if counts["mismatch"] or counts["error"]:
        if counts["mismatch"]:
            print(f"{counts['mismatch']} mismatch(es): a disagreement is a finding, not something to smooth over.")
        if counts["error"]:
            print(f"{counts['error']} error(s): justfetch failed to collect where the platform answered.")
        return 1

    if args.strict and counts["inconclusive"]:
        print(
            f"{counts['inconclusive']} covered module(s) had no comparison; "
            "strict mode does not accept that as evidence."
        )
        return 3

    if args.strict and gaps:
        print(
            f"{len(gaps)} module(s) had neither an observation nor a declaration "
            f"that they cannot be compared here: {', '.join(gaps)}. "
            "Strict mode treats that as missing evidence."
        )
        return 3

    # Acceptance needs at least one value the two sides actually agreed on.
    # Otherwise "everything was unsupported or unobserved" would pass as a run.
    if args.strict and counts["match"] == 0:
        print(
            "strict mode needs at least one confirmed match; this run compared "
            "nothing, so it is not acceptance evidence."
        )
        return 3

    if args.strict:
        print(
            f"Strict: {counts['match']} confirmed match(es); the rest were "
            "verified absent or unsupported."
        )
    else:
        print("Diagnostic: no disagreement found. 'inconclusive', 'not covered' and 'unsupported' are not passes; use --strict for acceptance.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
