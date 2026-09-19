# Metadata and provenance

Metadata is descriptive information that an application stores alongside your
project and media. It can include titles, descriptions, keywords, rights
information, camera notes, language-specific text, or fields defined by another
production application.

The same item can legitimately have several values. A clip might have several
keywords or titles in different languages. PostProject keeps those values
separate and does not silently pick one.

Some metadata belongs to the logical asset, while other metadata describes one
particular representation such as a camera original or proxy. PostProject keeps
that distinction so an application does not accidentally present
representation-specific facts as facts about every copy.

Provenance answers a different question: how was a result produced? The future
activity model will connect input representations, an operation and its output
representations. Descriptive metadata and production provenance can complement
each other, but neither is proof that content is authentic. Cryptographic trust
systems such as C2PA remain a separate layer.

All current metadata remains inside the local project file. PostProject does
not upload it, contact vocabulary services, or look up identifier registries.
