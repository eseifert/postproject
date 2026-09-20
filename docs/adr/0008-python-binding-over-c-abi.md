# ADR 0008: Python binding over the C ABI

- Status: Accepted
- Date: 2026-09-20

## Decision

The Python package will consume only the installed public C ABI, use explicit
native-library discovery and cleanup, and duplicate no domain validation. The
binding technology is the Python standard library's `ctypes` module.

The wrapper requires an explicit native-library path, either as an API argument
or through `POSTPROJECT_LIBRARY`. Paths are expanded and resolved before
loading; the wrapper does not search the current working directory or mutate
the platform loader path. Packagers may supply an absolute bundled-library path
from their own installation layout.

Every owned opaque C handle has one Python owner with deterministic `close()`
and context-manager cleanup plus an idempotent finalizer fallback. Borrowed
strings and result-set values are copied before the owning native handle is
released. Error codes become structured Python exceptions.

## Consequences

Python validates the same integration boundary used by non-Rust applications.
PyO3 and direct internal Rust access are excluded from the primary binding.

Using `ctypes` avoids an additional runtime or extension-compilation dependency
and works on Linux, macOS, and Windows. The tradeoff is a deliberately manual
signature table that must be kept synchronized with the public header and
exercised against the real shared library in CI.
