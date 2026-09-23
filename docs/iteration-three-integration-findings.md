# Iteration 3 integration findings

Iteration 3 began with four disposable integrations. Their repositories run
against PostProject `main`, and PostProject CI calls their reusable workflows
against the revision under test. They are probes, not compatibility promises.

## OpenAssetIO Manager spike

The [Manager spike](https://github.com/eseifert/postproject-openassetio-manager-spike)
proved that PostProject's HTTPS host-object binding can be used directly as an
OpenAssetIO entity reference. A Manager must keep a production open, translate
resolution failures into batch-element errors, and implement policy and trait
introspection even for a minimal read-only host. These findings led the
supported Manager to add existence queries, root mapping, and structure traits.

## OTIO-through-OpenAssetIO spike

The [OTIO spike](https://github.com/eseifert/postproject-otio-openassetio-spike)
composed the upstream OTIO media linker with the Manager without adding a
PostProject-specific OTIO plugin. OTIO retained rational time while the linker
changed only the external reference URL. The upstream linker currently handles
only `ExternalReference`; sequence structure must remain available through
OpenAssetIO traits until that mapping grows.

## Python host spike

The [Python host spike](https://github.com/eseifert/postproject-python-host-spike)
showed that installed applications need keyed collection access, an explicit
native-library path, short transaction scopes, and host bindings that survive
closing and reopening a production. The public Python API now demonstrates
those call sequences without exposing ctypes handles.

## C++ NLE spike

The [C++ NLE spike](https://github.com/eseifert/postproject-cpp-nle-spike)
links only the installed CMake package. An editor can preserve its own fallback
path beside a durable representation binding, compare a parsed binding, and
resolve the clip with copied C++ values. Ambiguity remains an application
decision rather than an implicit first-candidate choice.

## Maintained demonstrations

The disposable work informed the maintained
[PostProject OpenAssetIO Manager](https://github.com/eseifert/postproject-openassetio-manager)
and the runnable
[OTIO demonstration](https://github.com/eseifert/postproject-otio-demo).

