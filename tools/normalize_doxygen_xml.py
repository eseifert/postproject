"""Normalize Doxygen constexpr XML for Breathe compatibility."""

from __future__ import annotations

import argparse
import re
import xml.etree.ElementTree as ET
from pathlib import Path


_LEADING_CONSTEXPR = re.compile(r"^\s*constexpr\b\s*")


def normalize_file(path: Path) -> bool:
    """Remove redundant constexpr text already represented by Doxygen metadata."""
    tree = ET.parse(path)
    changed = False

    for member in tree.findall(".//memberdef[@kind='function'][@constexpr='yes']"):
        type_node = member.find("type")
        if type_node is None or type_node.text is None:
            continue

        normalized = _LEADING_CONSTEXPR.sub("", type_node.text, count=1)
        if normalized != type_node.text:
            type_node.text = normalized
            changed = True

    if changed:
        tree.write(path, encoding="utf-8", xml_declaration=True)
    return changed


def main() -> int:
    """Normalize every generated XML document in one Doxygen directory."""
    parser = argparse.ArgumentParser()
    parser.add_argument("xml_directory", type=Path)
    arguments = parser.parse_args()
    for path in arguments.xml_directory.glob("*.xml"):
        normalize_file(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
