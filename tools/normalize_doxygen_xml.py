"""Normalize Doxygen 1.16 constructor XML for Breathe compatibility."""

from __future__ import annotations

import argparse
import xml.etree.ElementTree as ET
from pathlib import Path


def normalize_file(path: Path) -> bool:
    """Remove the duplicated constructor type emitted by Doxygen 1.16."""
    tree = ET.parse(path)
    changed = False
    for member in tree.findall(".//memberdef[@kind='function'][@constexpr='yes']"):
        type_node = member.find("type")
        name = member.findtext("name")
        definition = member.findtext("definition")
        if (
            type_node is not None
            and type_node.text == "constexpr"
            and name
            and definition
            and definition.endswith(f"::{name}")
        ):
            type_node.text = ""
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
