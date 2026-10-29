#!/usr/bin/env python3
"""Compare a justfetch document with observations from the platform's own tools.

Used by scripts/verify-*. The platform script writes both files:

  doc.json    the output of `justfetch --format json --modules ...`
  native.json {
                "tools":  {"battery": "pmset", ...},
                "values": {"battery": {"percent": 87, "state": "discharging"},
                           "packages": {"homebrew formulae": 883},
                           "gpu": ["Apple M3"], "display": ["1470x956"],
                           "desktop": "Aqua", "locale": "zh_CN.UTF-8"}
              }

Each module gets one verdict:

  match         the two agree
  mismatch      they disagree — the interesting case
  uncomparable  the platform tool gave nothing to compare against
  unsupported   justfetch says the question does not apply, and the platform
                tool agrees there is nothing

Exit status: 0 when nothing mismatched, 1 when something did, 2 on bad input.
Only non-identifying values are printed: counts, resolutions, names, locales.
"""

import argparse
import json
import sys

MODULES = ["gpu", "battery", "display", "packages", "desktop", "locale"]


def show(value, limit=34):
    text = json.dumps(value, ensure_ascii=False) if not isinstance(value, str) else value
    return text if len(text) <= limit else text[: limit - 1] + "…"


def battery(value):
    units = value.get("units", [])
    if not units:
        return ""
    first = units[0]
    percent = "?" if first.get("percent") is None else f"{first['percent']}%"
    return f"{percent} {first.get('state', '?')}"


def packages(value):
    return ", ".join(f"{m['name']} {m['count']}" for m in value.get("managers", []))


def gpu(value):
    return ", ".join(
        device.get("name") or f"{device.get('vendor', '?')} {device.get('id', '')}".strip()
        for device in value.get("devices", [])
    )


def display(value):
    return ", ".join(
        f"{unit['width']}x{unit['height']}" for unit in value.get("displays", [])
    )


def locale(value):
    return value.get("process") or value.get("system") or ""


SHOWN = {
    "battery": battery,
    "packages": packages,
    "gpu": gpu,
    "display": display,
    "desktop": lambda value: value.get("name", ""),
    "locale": locale,
}


def verdict(module, status, value, native):
    """Return (verdict, native-text) for one module."""
    if native is None:
        if status == "unsupported":
            return "unsupported", "—"
        if status == "unavailable":
            # The tool could not answer either; that is not "not applicable".
            return "uncomparable", "—"
        return "uncomparable", "—"

    if status in ("unsupported", "unavailable"):
        # justfetch declined but the platform tool has an answer.
        return "mismatch", show(native)

    if module == "battery":
        if not isinstance(native, dict):
            return "uncomparable", show(native)
        units = value.get("units", [])
        percent = units[0].get("percent") if units else None
        state = units[0].get("state") if units else None
        ok = percent == native.get("percent") and (
            native.get("state") is None or state == native["state"]
        )
        return ("match" if ok else "mismatch"), show(native)

    if module == "packages":
        ours = {m["name"]: m["count"] for m in value.get("managers", [])}
        theirs = {k: v for k, v in native.items()}
        ok = all(ours.get(name) == count for name, count in theirs.items())
        return ("match" if ok else "mismatch"), show(theirs)

    if module == "gpu":
        names = [str(n).lower() for n in native]
        ours = " ".join(gpu(value).split()).lower()
        ok = any(name and name in ours for name in names)
        return ("match" if ok else "mismatch"), show(native)

    if module == "display":
        resolutions = {str(r) for r in native}
        ours = {f"{u['width']}x{u['height']}" for u in value.get("displays", [])}
        return ("match" if resolutions & ours else "mismatch"), show(native)

    if module in ("desktop", "locale"):
        ours = SHOWN[module](value).lower()
        theirs = str(native).lower()
        ok = bool(theirs) and (theirs in ours or ours in theirs)
        return ("match" if ok else "mismatch"), show(native)

    return "uncomparable", show(native)


def check_inputs(document, native):
    """Return a description of the first malformed input, or None.

    The exit status distinguishes a mismatch (1) from a bad input (2); an
    exception would be reported as 1 and look like a disagreement.
    """
    if not isinstance(document, dict):
        return "the justfetch document is not an object"
    for module in MODULES:
        field = document.get(module)
        if not isinstance(field, dict) or "status" not in field:
            return f"`{module}` is missing or not an object"
        if field["status"] == "available" and not isinstance(field.get("value"), dict):
            return f"`{module}` says available without a value object"
    values = native.get("values", {}) if isinstance(native, dict) else {}
    if not isinstance(values, dict):
        return "native `values` is not an object"
    shapes = {"battery": dict, "packages": dict, "gpu": list, "display": list,
              "desktop": str, "locale": str}
    for module, expected in shapes.items():
        value = values.get(module)
        if value is not None and not isinstance(value, expected):
            return f"native `{module}` is {type(value).__name__}, expected {expected.__name__}"
    if isinstance(values.get("gpu"), list) and not all(isinstance(n, str) for n in values["gpu"]):
        return "native `gpu` must be a list of strings"
    if isinstance(values.get("packages"), dict) and not all(
            isinstance(count, int) for count in values["packages"].values()):
        return "native `packages` must map names to integers"
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--justfetch", required=True)
    parser.add_argument("--native", required=True)
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

    print(f"{'module':10} {'justfetch':34} {'platform tool':34} verdict")
    print("-" * 92)
    mismatches = 0
    for module in MODULES:
        field = document.get(module, {})
        status = field.get("status")
        shown = SHOWN[module](field["value"]) if status == "available" else status
        result, native_text = verdict(module, status, field.get("value", {}), values.get(module))
        tool = tools.get(module)
        native_text = f"{native_text} ({tool})" if tool and native_text != "—" else native_text
        if result == "mismatch":
            mismatches += 1
        print(f"{module:10} {show(shown):34} {native_text:34} {result}")

    print("-" * 92)
    if mismatches:
        print(f"{mismatches} mismatch(es): a disagreement is a finding, not something to smooth over.")
        return 1
    print("No comparable value disagreed. 'uncomparable' and 'unsupported' are not passes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
