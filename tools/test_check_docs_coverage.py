"""Unit tests for C reference coverage enforcement."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "check_docs_coverage", ROOT / "tools" / "check_docs_coverage.py"
)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class DocumentationCoverageTests(unittest.TestCase):
    """Exercise multiline declarations and Doxygen XML extraction."""

    def test_finds_exported_function_declarations(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "api.h"
            header.write_text(
                "PP_API int pp_first(void);\n"
                "PP_API const char *\npp_second(int value);\n"
                "int private_function(void);\n",
                encoding="utf-8",
            )
            self.assertEqual(
                CHECKER.exported_functions(header), {"pp_first", "pp_second"}
            )

    def test_reads_only_function_members(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            xml_directory = Path(directory)
            (xml_directory / "api.xml").write_text(
                "<doxygen><compounddef><sectiondef>"
                "<memberdef kind='function'><name>pp_first</name></memberdef>"
                "<memberdef kind='variable'><name>pp_not_a_function</name></memberdef>"
                "</sectiondef></compounddef></doxygen>",
                encoding="utf-8",
            )
            self.assertEqual(CHECKER.documented_functions(xml_directory), {"pp_first"})


if __name__ == "__main__":
    unittest.main()
