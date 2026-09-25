# Release 0.3 integration findings

Release 0.3 began with four disposable integrations. Their repositories run
against PostProject `main`, and PostProject CI calls their reusable workflows
against the revision under test. They are probes, not compatibility promises.

## OpenAssetIO Manager validation

The [Manager validation](https://github.com/postproject-org/postproject-openassetio)
proved that PostProject's HTTPS host-object binding can be used directly as an
OpenAssetIO entity reference. A Manager must keep a production open, translate
resolution failures into batch-element errors, and implement policy and trait
introspection even for a minimal read-only host. These findings led the
supported Manager to add existence queries, root mapping, and structure traits.

## OTIO-through-OpenAssetIO validation

The [OTIO validation](https://github.com/postproject-org/postproject-otio-openassetio)
composed the upstream OTIO media linker with the Manager without adding a
PostProject-specific OTIO plugin. OTIO retained rational time while the linker
changed only the external reference URL. The upstream linker currently handles
only `ExternalReference`; sequence structure must remain available through
OpenAssetIO traits until that mapping grows.

## Python host validation

The [Python host validation](https://github.com/postproject-org/postproject-python-host)
showed that installed applications need keyed collection access, an explicit
native-library path, short transaction scopes, and host bindings that survive
closing and reopening a production. The public Python API now demonstrates
those call sequences without exposing ctypes handles.

## C++ NLE validation

The [C++ NLE validation](https://github.com/postproject-org/postproject-cpp-nle)
links only the installed CMake package. An editor can preserve its own fallback
path beside a durable representation binding, compare a parsed binding, and
resolve the clip with copied C++ values. Ambiguity remains an application
decision rather than an implicit first-candidate choice.

## Maintained demonstrations

The disposable work informed the maintained
[PostProject OpenAssetIO Manager](https://github.com/postproject-org/postproject-openassetio-manager)
and the runnable
[OTIO demonstration](https://github.com/postproject-org/postproject-otio-demo).
