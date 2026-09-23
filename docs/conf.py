"""Sphinx configuration for the unified PostProject documentation site."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python" / "src"))

project = "PostProject"
author = "PostProject contributors"
release = "0.3.0-alpha.1"
version = "0.3"

extensions = ["myst_parser", "breathe", "sphinx.ext.autodoc"]
myst_enable_extensions = ["colon_fence", "deflist"]
source_suffix = {".rst": "restructuredtext", ".md": "markdown"}
root_doc = "index"
exclude_patterns = ["_build", "src/SUMMARY.md", "src/index.md"]

breathe_projects = {"PostProject": str(ROOT / "target" / "doxygen" / "xml")}
breathe_default_project = "PostProject"
breathe_domain_by_extension = {"h": "c", "hpp": "cpp"}

html_theme = "furo"
html_title = f"PostProject {release} · ABI 15"
html_static_path = ["_static"]
html_css_files = ["postproject.css"]
templates_path = ["_templates"]
html_extra_path = ["CNAME", "versions.json"]
html_theme_options = {
    "source_repository": "https://github.com/eseifert/postproject/",
    "source_branch": "main",
    "source_directory": "docs/",
}
html_sidebars = {
    "**": [
        "sidebar/brand.html",
        "sidebar/search.html",
        "version-switcher.html",
        "language-switcher.html",
        "sidebar/scroll-start.html",
        "sidebar/navigation.html",
        "sidebar/scroll-end.html",
    ]
}
