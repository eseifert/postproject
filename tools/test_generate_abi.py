from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "generate_abi", ROOT / "tools" / "generate_abi.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


class GenerateAbiTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.header_path = ROOT / "include" / "postproject" / "postproject.h"
        cls.header = GENERATOR.parse_header(cls.header_path.read_text(encoding="utf-8"))

    def test_symbol_output_matches_allowlist(self) -> None:
        expected = (ROOT / "tests" / "abi" / "expected-symbols.txt").read_text(
            encoding="utf-8"
        )
        self.assertEqual(GENERATOR.render_symbols(self.header), expected)

    def test_python_output_matches_committed_module(self) -> None:
        first = GENERATOR.render_python(self.header, str(self.header_path.relative_to(ROOT)))
        second = GENERATOR.render_python(self.header, str(self.header_path.relative_to(ROOT)))
        self.assertEqual(first, second)
        compile(first, "_abi.py", "exec")
        committed = (ROOT / "python" / "src" / "postproject" / "_abi.py").read_text(
            encoding="utf-8"
        )
        self.assertEqual(first, committed)

    def test_unsupported_declaration_fails_loudly(self) -> None:
        source = """
        typedef uint32_t pp_value_t;
        typedef union pp_choice { uint32_t value; } pp_choice_t;
        PP_API void pp_use_choice(pp_choice_t value);
        """
        with self.assertRaisesRegex(GENERATOR.HeaderError, "unsupported"):
            GENERATOR.parse_header(source)


if __name__ == "__main__":
    unittest.main()
