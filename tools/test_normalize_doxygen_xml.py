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
    """Ensure redundant constexpr text is removed without changing return types."""

    def _normalize(self, member_xml: str) -> ET.Element:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "class.xml"
            path.write_text(f"<doxygen>{member_xml}</doxygen>", encoding="utf-8")
            self.assertTrue(NORMALIZER.normalize_file(path))
            return ET.parse(path).getroot()

    def test_normalizes_constexpr_constructor(self) -> None:
        root = self._normalize(
            "<memberdef kind='function' constexpr='yes'>"
            "<type>constexpr</type><definition>example::Value::Value</definition>"
            "<name>Value</name></memberdef>"
        )
        self.assertEqual(root.findtext(".//type"), "")

    def test_normalizes_constexpr_constructor_with_whitespace(self) -> None:
        root = self._normalize(
            "<memberdef kind='function' constexpr='yes'>"
            "<type> constexpr </type><definition>example::Value::Value</definition>"
            "<name>Value</name></memberdef>"
        )
        self.assertEqual(root.findtext(".//type"), "")

    def test_preserves_return_type_after_constexpr(self) -> None:
        root = self._normalize(
            "<memberdef kind='function' constexpr='yes'>"
            "<type>constexpr int</type><definition>example::Value::size</definition>"
            "<name>size</name></memberdef>"
        )
        self.assertEqual(root.findtext(".//type"), "int")

    def test_leaves_non_constexpr_member_unchanged(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "class.xml"
            original = (
                "<doxygen><memberdef kind='function' constexpr='no'>"
                "<type>constexpr_like</type><definition>example::Value::size</definition>"
                "<name>size</name></memberdef></doxygen>"
            )
            path.write_text(original, encoding="utf-8")
            self.assertFalse(NORMALIZER.normalize_file(path))
            self.assertEqual(path.read_text(encoding="utf-8"), original)


if __name__ == "__main__":
    unittest.main()
