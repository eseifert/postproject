"""Unit tests for the Doxygen XML compatibility normalization."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "normalize_doxygen_xml", ROOT / "tools" / "normalize_doxygen_xml.py"
)
assert SPEC is not None and SPEC.loader is not None
NORMALIZER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = NORMALIZER
SPEC.loader.exec_module(NORMALIZER)


class DoxygenNormalizationTests(unittest.TestCase):
    """Ensure only the duplicated constexpr constructor type is removed."""

    def test_normalizes_constexpr_constructor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "class.xml"
            path.write_text(
                "<doxygen><memberdef kind='function' constexpr='yes'>"
                "<type>constexpr</type><definition>example::Value::Value</definition>"
                "<name>Value</name></memberdef></doxygen>",
                encoding="utf-8",
            )
            self.assertTrue(NORMALIZER.normalize_file(path))
            self.assertEqual(ET.parse(path).findtext(".//type"), "")


if __name__ == "__main__":
    unittest.main()
