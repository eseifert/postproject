"""Normalize Doxygen constexpr XML for Breathe compatibility."""

from __future__ import annotations

import argparse
import re
import xml.etree.ElementTree as ET
from pathlib import Path


_LEADING_CONSTEXPR = re.compile(r"^\s*constexpr\b")


def _starts_with_constexpr(node: ET.Element) -> bool:
    """Return whether linked Doxygen text begins with the constexpr keyword."""
    return _LEADING_CONSTEXPR.match("".join(node.itertext())) is not None


def normalize_file(path: Path) -> bool:
    """Avoid representing constexpr twice in XML consumed by Breathe."""
    tree = ET.parse(path)
    changed = False

    for member in tree.findall(".//memberdef[@kind='function'][@constexpr='yes']"):
        type_node = member.find("type")
        if type_node is not None and _starts_with_constexpr(type_node):
            # Doxygen can encode constexpr both as member metadata and in the
            # rendered type. Breathe renders both, producing `constexpr
            # constexpr`. Keep the textual declaration and disable the
            # duplicate metadata representation.
            member.set("constexpr", "no")
            changed = True

    if changed:
        tree.write(path, encoding="utf-8", xml_declaration=True)
    return changed


def main() -> int:
    """Normalize every generated XML document in one Doxygen directory."""
    parser = argparse.ArgumentParser()
    parser.add_argument("xml_directory", type=Path)
    arguments = parser.parse_args()
    changed_files = sum(
        normalize_file(path) for path in arguments.xml_directory.glob("*.xml")
    )
    print(f"normalized {changed_files} Doxygen XML file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
