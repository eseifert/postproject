"""Fail when an exported C function is absent from generated Doxygen XML."""

from __future__ import annotations

import argparse
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def exported_functions(header: Path) -> set[str]:
    """Return PP_API function names declared by the authoritative header."""
    text = header.read_text(encoding="utf-8")
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    declarations = re.findall(r"\bPP_API\b(.*?);", text, flags=re.DOTALL)
    names = set()
    for declaration in declarations:
        match = re.search(r"\b(pp_[a-z0-9_]+)\s*\(", declaration)
        if match:
            names.add(match.group(1))
    return names


def documented_functions(xml_directory: Path) -> set[str]:
    """Return function names present in Doxygen's generated XML members."""
    names = set()
    for path in xml_directory.glob("*.xml"):
        root = ET.parse(path).getroot()
        for member in root.findall(".//memberdef[@kind='function']/name"):
            if member.text:
                names.add(member.text)
    return names


def main() -> int:
    """Check the generated reference and print actionable missing symbols."""
    parser = argparse.ArgumentParser()
    parser.add_argument("header", type=Path)
    parser.add_argument("xml_directory", type=Path)
    arguments = parser.parse_args()
    missing = sorted(
        exported_functions(arguments.header)
        - documented_functions(arguments.xml_directory)
    )
    if missing:
        print("exported C functions missing from the generated reference:")
        for name in missing:
            print(f"  {name}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
