"""Programming-language variants for cross-surface code examples.

``code-variants`` renders one example per public surface as sphinx-design tabs
in one synchronized group, so the sidebar code-language selector and any tab
click switch every example on the site.

With a region argument, each variant is read from the tested example program
of its surface (``postproject_code_examples``), between the marker comments
``[region]`` and ``[/region]``. The block content may add inline code blocks
and must explain every surface without a region using ``no-variant``; an
unexplained surface is a build warning, so CI notices a missing example.
"""

from __future__ import annotations

import re
import textwrap
from pathlib import Path

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.util import logging
from sphinx.util.docutils import SphinxDirective
from sphinx_design.shared import create_component

LOGGER = logging.getLogger(__name__)

SYNC_GROUP = "code-language"

# Canonical surfaces in display order: (sync id, tab label, highlight language).
SURFACES = (
    ("c", "C", "c"),
    ("cpp", "C++", "cpp"),
    ("python", "Python", "python"),
    ("rust", "Rust", "rust"),
    ("cli", "CLI", "bash"),
)
LABELS = {surface: label for surface, label, _ in SURFACES}
HIGHLIGHT = {surface: language for surface, _, language in SURFACES}
ORDER = {surface: index for index, (surface, _, _) in enumerate(SURFACES)}

# Highlighting languages accepted for inline code blocks of each surface.
LANGUAGE_SURFACES = {
    "c": "c",
    "cpp": "cpp",
    "c++": "cpp",
    "python": "python",
    "py": "python",
    "rust": "rust",
    "sh": "cli",
    "shell": "cli",
    "bash": "cli",
    "console": "cli",
}

MARKER = re.compile(r"^\s*(?://|#|/\*)\s+\[(/?)([a-z0-9-]+)\]\s*(?:\*/)?\s*$")


class no_variant(nodes.container):
    """Explains why one surface has no equivalent operation."""


class NoVariantDirective(SphinxDirective):
    """Marks a surface as deliberately lacking an equivalent operation."""

    required_arguments = 1
    has_content = True

    def run(self) -> list[nodes.Node]:
        surface = self.arguments[0]
        if surface not in LABELS:
            raise self.error(
                f"unknown surface {surface!r}; expected one of {', '.join(LABELS)}"
            )
        self.assert_has_content()
        node = no_variant(classes=["pp-no-variant"], surface=surface)
        self.set_source_info(node)
        self.state.nested_parse(self.content, self.content_offset, node)
        return [node]


def read_regions(path: Path) -> dict[str, str]:
    """Return every marked region of an example program, dedented."""
    regions: dict[str, str] = {}
    open_name: str | None = None
    lines: list[str] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = MARKER.match(line)
        if match is None:
            if open_name is not None:
                lines.append(line)
            continue
        closing, name = match.groups()
        if not closing:
            if open_name is not None:
                raise ValueError(
                    f"{path}:{number}: region {name!r} opened inside {open_name!r}"
                )
            if name in regions:
                raise ValueError(f"{path}:{number}: duplicate region {name!r}")
            open_name, lines = name, []
        elif name != open_name:
            raise ValueError(f"{path}:{number}: unexpected end of region {name!r}")
        else:
            regions[name] = textwrap.dedent("\n".join(lines)).strip("\n")
            open_name = None
    if open_name is not None:
        raise ValueError(f"{path}: region {open_name!r} is not closed")
    return regions


class CodeVariantsDirective(SphinxDirective):
    """Groups per-surface examples into synchronized tabs."""

    optional_arguments = 1
    has_content = True

    def run(self) -> list[nodes.Node]:
        variants: dict[str, nodes.Element] = {}
        if self.arguments:
            self.add_region_variants(self.arguments[0], variants)
        if self.content:
            container = nodes.container()
            self.state.nested_parse(self.content, self.content_offset, container)
            for child in container.children:
                self.add_child_variant(child, variants)

        missing = [label for surface, label, _ in SURFACES if surface not in variants]
        if missing:
            self.warn(
                "missing variants for "
                + ", ".join(missing)
                + "; add a tested example region or a no-variant explanation"
            )

        tab_set = create_component(
            "tab-set", classes=["sd-tab-set", "pp-code-variants"]
        )
        self.set_source_info(tab_set)
        for surface in sorted(variants, key=ORDER.__getitem__):
            label = nodes.rubric(LABELS[surface], nodes.Text(LABELS[surface]))
            label["classes"] = ["sd-tab-label"]
            label["sync_group"] = SYNC_GROUP
            label["sync_id"] = surface
            content = create_component(
                "tab-content", children=[variants[surface]], classes=["sd-tab-content"]
            )
            tab_set += create_component(
                "tab-item", children=[label, content], classes=["sd-tab-item"]
            )
        return [tab_set]

    def add_region_variants(self, region: str, variants: dict[str, nodes.Element]):
        confdir = Path(self.env.app.confdir)
        for surface, relative in self.config.postproject_code_examples.items():
            path = confdir / relative
            self.env.note_dependency(str(path))
            try:
                text = read_regions(path).get(region)
            except (OSError, ValueError) as error:
                self.warn(str(error))
                continue
            if text is None:
                continue
            block = nodes.literal_block(text, text, language=HIGHLIGHT[surface])
            self.set_source_info(block)
            variants[surface] = block

    def add_child_variant(self, child: nodes.Node, variants: dict[str, nodes.Element]):
        if isinstance(child, nodes.literal_block):
            language = child.get("language", "")
            surface = LANGUAGE_SURFACES.get(language)
            if surface is None:
                self.warn(f"code block language {language!r} maps to no surface")
                return
        elif isinstance(child, no_variant):
            surface = child["surface"]
        else:
            self.warn("code-variants content accepts only code blocks and no-variant")
            return
        if surface in variants:
            self.warn(f"{LABELS[surface]} has both an example and another variant")
            return
        variants[surface] = child

    def warn(self, message: str) -> None:
        LOGGER.warning(message, location=self.get_location(), type="postproject")


def visit_no_variant(translator, node: no_variant) -> None:
    translator.visit_container(node)


def depart_no_variant(translator, node: no_variant) -> None:
    translator.depart_container(node)


def setup(app: Sphinx) -> dict[str, object]:
    app.setup_extension("sphinx_design")
    app.add_config_value("postproject_code_examples", {}, "env", dict)
    handlers = (visit_no_variant, depart_no_variant)
    app.add_node(no_variant, html=handlers, latex=handlers, text=handlers)
    app.add_directive("code-variants", CodeVariantsDirective)
    app.add_directive("no-variant", NoVariantDirective)
    return {"parallel_read_safe": True, "parallel_write_safe": True}
