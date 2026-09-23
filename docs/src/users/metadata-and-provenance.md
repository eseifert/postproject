# Metadata and provenance

Metadata is descriptive information that an application stores alongside your
production and media. It can include titles, descriptions, keywords, rights
information, camera notes, language-specific text, or fields defined by another
production application.

The same item can legitimately have several values. A clip might have several
keywords or titles in different languages. PostProject keeps those values
separate and does not silently pick one.

Some metadata belongs to the logical asset, while other metadata describes one
particular representation such as a camera original or proxy. PostProject keeps
that distinction so an application does not accidentally present
representation-specific facts as facts about every copy.

The CLI can optionally inspect imported files with an installed `ffprobe` and
store container, stream, codec, dimensions, rates, channel layout, bit depth,
pixel format, duration, timecode, and other embedded tags as typed metadata:

```sh
postproject media add production.pproj camera.mov --inspect
```

Inspection is optional. If `ffprobe` is missing or rejects the file, import
still succeeds and the command reports `unavailable` or `failed` rather than
inventing technical values.

Provenance answers a different question: how was a result produced? An activity
connects input representations, an operation, and its output representations.
It can also record the responsible tool, version, agent, timing, and processing
parameters. Descriptive metadata and production provenance can complement each
other, but neither is proof that content is authentic. Cryptographic trust
systems such as C2PA remain a separate layer.

All current metadata remains inside the local production file. PostProject does
not upload it, contact vocabulary services, or look up identifier registries.
