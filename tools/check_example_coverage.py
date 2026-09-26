#!/usr/bin/env python3
"""Check that tested documentation examples exercise every public operation.

The documentation promises that each public operation can be seen in action on
every surface that offers it. This check enforces that promise for the three
surfaces whose operations can be listed mechanically:

* every function declared in the C header appears in a C example;
* every public member function of a C++ wrapper class appears in a C++ example;
* every public method and property of the Python handle classes appears in a
  Python example.

Names are matched as whole identifiers, so a call anywhere in an example
program counts. Exit status 1 lists every uncovered operation.
"""

from __future__ import annotations

import argparse
import ast
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HEADER = ROOT / "include" / "postproject" / "postproject.h"
WRAPPER = ROOT / "include" / "postproject" / "postproject.hpp"
PYTHON_PRODUCTION = ROOT / "python" / "src" / "postproject" / "_production.py"
EXAMPLES = ROOT / "docs" / "examples"

# Operations that have no meaningful demonstration of their own.
C_EXEMPT = {
    # Called implicitly by every example through its error-reporting path.
    "pp_abi_version",
}
CPP_EXEMPT = {
    # Special members and conversions.
    "operator",
}
PYTHON_EXEMPT: set[str] = set()


def c_functions(header: str) -> set[str]:
    source = re.sub(r"/\*.*?\*/", "", header, flags=re.DOTALL)
    return set(re.findall(r"\bPP_API\b[^;(]*?\b(pp_\w+)\s*\(", source, flags=re.DOTALL))


def cpp_methods(wrapper: str) -> dict[str, set[str]]:
    """Return public member functions of each wrapper class, by class."""
    methods: dict[str, set[str]] = {}
    lines = wrapper.splitlines()
    index = 0
    while index < len(lines):
        match = re.match(r"^class (\w+) final\b", lines[index])
        if match is None:
            index += 1
            continue
        name = match.group(1)
        public = False
        depth = lines[index].count("{") - lines[index].count("}")
        found: set[str] = set()
        index += 1
        while index < len(lines) and (depth > 0 or "{" not in lines[index - 1]):
            line = lines[index]
            if line.startswith("public:"):
                public = True
            elif line.startswith(("private:", "protected:")):
                public = False
            elif public and depth == 1:
                declaration = re.match(
                    r"^  (?:\[\[nodiscard\]\] )?(?:static |explicit |inline )*"
                    r"(?:[\w:<>,*& ]+?\s+)?[*&]?(\w+)\(",
                    line,
                )
                if (
                    declaration
                    and declaration.group(1) not in {name, "~" + name}
                    and not declaration.group(1).endswith("_")
                ):
                    found.add(declaration.group(1))
            depth += line.count("{") - line.count("}")
            index += 1
            if depth <= 0:
                break
        methods[name] = found
    return methods


def python_members(source: str) -> dict[str, set[str]]:
    tree = ast.parse(source)
    members: dict[str, set[str]] = {}
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and not node.name.startswith("_"):
            members[node.name] = {
                item.name
                for item in node.body
                if isinstance(item, ast.FunctionDef) and not item.name.startswith("_")
            }
    return members


def identifiers(paths: list[Path]) -> set[str]:
    words: set[str] = set()
    for path in paths:
        words.update(re.findall(r"\b\w+\b", path.read_text(encoding="utf-8")))
    return words


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.parse_args()

    missing: list[str] = []

    c_used = identifiers(sorted(EXAMPLES.glob("c/*.c")))
    for function in sorted(c_functions(HEADER.read_text(encoding="utf-8")) - C_EXEMPT):
        if function not in c_used:
            missing.append(f"C: {function}")

    cpp_used = identifiers(sorted(EXAMPLES.glob("cpp/*.cpp")))
    for class_name, methods in sorted(
        cpp_methods(WRAPPER.read_text(encoding="utf-8")).items()
    ):
        for method in sorted(methods - CPP_EXEMPT):
            if method not in cpp_used:
                missing.append(f"C++: {class_name}::{method}")

    python_used = identifiers(sorted(EXAMPLES.glob("python/*.py")))
    members = python_members(PYTHON_PRODUCTION.read_text(encoding="utf-8"))
    for class_name, names in sorted(members.items()):
        for name in sorted(names - PYTHON_EXEMPT):
            if name not in python_used:
                missing.append(f"Python: {class_name}.{name}")

    if missing:
        print("operations without a tested documentation example:", file=sys.stderr)
        for item in missing:
            print(f"  {item}", file=sys.stderr)
        return 1
    print("every public C, C++, and Python operation has a tested example")
    return 0


if __name__ == "__main__":
    sys.exit(main())
