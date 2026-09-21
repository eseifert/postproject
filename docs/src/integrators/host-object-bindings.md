# Host-object bindings

An editor, scene application, or automation host usually keeps its own document
format. When that document refers to PostProject knowledge, persist the complete
production-scoped identity rather than an object UUID by itself:

```text
postproject:v1:<production UUID>:<object kind>:<object UUID>
```

The supported object-kind tokens are `production`, `asset`, `representation`,
`resource`, and `activity`. Treat the serialized value as opaque text. It is
versioned, but it is not a registered URI scheme and must not be resolved over a
network.

Rust integrations can construct and parse the value with `HostObjectBinding`:

```rust
use std::str::FromStr;

use postproject_core::{HostObjectBinding, ObjectRef};

let binding = HostObjectBinding::new(
    production_id,
    ObjectRef::Representation(representation_id),
)
.expect("valid binding");
let stored = binding.to_string();

let reopened = HostObjectBinding::from_str(&stored).expect("valid stored binding");
assert_eq!(reopened, binding);
```

Parsing is deliberately strict: versions and object kinds must be known, UUIDs
must use lowercase hyphenated canonical text, and extra fields are rejected. A
future format can therefore be introduced without interpreting ambiguous old
text.

## Fallback information

A host may separately retain a display name, production path, or last known
locator to help a person repair an unavailable binding. That fallback is never
part of identity. If the production cannot be opened or the object does not
exist, preserve the binding and report an explicit rebind state; do not silently
select a production or object from fallback text.

## The opposite direction

When PostProject needs to find an object owned by the host, attach the host's
stable identifier through the external-identifier API. Host identifiers are
lookup aids and assertions. They do not become globally unique merely because a
host supplied them, and they do not replace the production-scoped binding above.
