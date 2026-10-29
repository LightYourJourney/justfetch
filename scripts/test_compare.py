#!/usr/bin/env python3
"""Regression tests for scripts/compare.py.

Run with `python3 scripts/test_compare.py`. The cases are the ones the review
rounds found: a BOM from Windows PowerShell, malformed input that must exit 2
rather than 1, a deselected module that must not be reported as a match, and
fabricated extra data that a one-directional comparison would accept.
"""

import copy
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
COMPARE = HERE / "compare.py"
MODULES = ["gpu", "battery", "display", "packages", "desktop", "locale"]


def document(**overrides):
    # Deep copies: a test that mutates a value must not corrupt the next one.
    fields = {module: {"status": "deselected"} for module in MODULES}
    fields.update(copy.deepcopy(overrides))
    return {"schema_version": 1, "identity": {}, "environment": {"kind": "native"}, **fields}


AVAILABLE = {
    "gpu": {"status": "available", "value": {"devices": [{"name": "Apple M3"}]},
            "source": "x", "confidence": "confirmed"},
    "battery": {"status": "available",
                "value": {"units": [{"name": "B", "percent": 80, "state": "discharging"}],
                          "power": "battery"}, "source": "x", "confidence": "confirmed"},
    "display": {"status": "available",
                "value": {"displays": [{"width": 1470, "height": 956}], "definition": "x"},
                "source": "x", "confidence": "confirmed"},
    "packages": {"status": "available",
                 "value": {"managers": [{"name": "homebrew formulae", "count": 883}]},
                 "source": "x", "confidence": "confirmed"},
    "desktop": {"status": "unsupported", "reason": "macOS"},
    "locale": {"status": "available",
               "value": {"process": "zh_CN.UTF-8"}, "source": "x", "confidence": "confirmed"},
}

NATIVE = {
    "tools": {},
    "values": {
        "gpu": {"names": ["Apple M3"]},
        "battery": {"percent": 80, "state": "discharging"},
        "display": ["1470x956"],
        "packages": {"homebrew formulae": 883},
        "locale": "zh_CN.UTF-8",
    },
}


class Compare(unittest.TestCase):
    def run_compare(self, doc, native, bom=False):
        with tempfile.TemporaryDirectory() as work:
            doc_path = pathlib.Path(work, "doc.json")
            native_path = pathlib.Path(work, "native.json")
            doc_path.write_text(json.dumps(doc))
            if bom:
                native_path.write_bytes(b"\xef\xbb\xbf" + json.dumps(native).encode())
            else:
                native_path.write_text(json.dumps(native))
            result = subprocess.run(
                [sys.executable, str(COMPARE), "--justfetch", str(doc_path),
                 "--native", str(native_path)],
                capture_output=True, text=True)
        return result

    def test_agreement_exits_zero(self):
        result = self.run_compare(document(**AVAILABLE), NATIVE)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("match", result.stdout)

    def test_a_disagreement_exits_one(self):
        native = json.loads(json.dumps(NATIVE))
        native["values"]["battery"]["percent"] = 12
        result = self.run_compare(document(**AVAILABLE), native)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_a_bom_is_accepted(self):
        result = self.run_compare(document(**AVAILABLE), NATIVE, bom=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_malformed_tools_is_bad_input(self):
        native = json.loads(json.dumps(NATIVE))
        native["tools"] = []
        result = self.run_compare(document(**AVAILABLE), native)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_a_manager_without_a_name_is_bad_input(self):
        doc = document(**AVAILABLE)
        doc["packages"]["value"]["managers"] = [{"count": 3}]
        result = self.run_compare(doc, NATIVE)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_a_gpu_given_as_a_string_is_bad_input(self):
        native = json.loads(json.dumps(NATIVE))
        native["values"]["gpu"] = "Apple M3"
        result = self.run_compare(document(**AVAILABLE), native)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_a_fabricated_extra_adapter_is_a_mismatch(self):
        doc = document(**AVAILABLE)
        doc["gpu"]["value"]["devices"].append({"name": "Fake GPU"})
        result = self.run_compare(doc, NATIVE)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_a_fabricated_extra_manager_is_a_mismatch(self):
        doc = document(**AVAILABLE)
        doc["packages"]["value"]["managers"].append({"name": "bogus", "count": 1})
        result = self.run_compare(doc, NATIVE)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_a_deselected_module_is_not_a_match(self):
        native = {"tools": {}, "values": {"desktop": "sway"}}
        result = self.run_compare(document(), native)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        desktop = [line for line in result.stdout.splitlines() if line.startswith("desktop")][0]
        self.assertIn("uncomparable", desktop)
        self.assertNotIn("match", desktop)

    def test_unsupported_without_a_native_answer_is_unsupported(self):
        result = self.run_compare(document(), {"tools": {}, "values": {}})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("unsupported", result.stdout)

    def test_unsupported_with_a_native_answer_is_a_mismatch(self):
        doc = document(battery={"status": "unsupported", "reason": "no battery"})
        native = {"tools": {}, "values": {"battery": {"percent": 50, "state": "full"}}}
        result = self.run_compare(doc, native)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=1)
