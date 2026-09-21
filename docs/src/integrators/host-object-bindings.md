# Host-object bindings

An editor, scene application, or automation host usually keeps its own document
format. When that document refers to PostProject knowledge, persist the complete
production-scoped identity rather than an object UUID by itself:

```text
https://postproject.org/ref/v1/<production UUID>/<object kind>/<object UUID>
```

The supported object-kind tokens are `production`, `asset`, `representation`,
`resource`, and `activity`. Treat the serialized value as opaque identity text.
Its project-controlled HTTPS namespace can point to documentation, but parsing
and using a binding never performs a network request.

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

The public C ABI exposes `pp_host_binding_format` and
`pp_host_binding_parse`. A formatted string is caller-owned and must be
released exactly once with `pp_host_binding_release`; parsed UUID and object
reference values are copied into caller-owned output structs.

C++ exposes the same operations as a copied value:

```cpp
postproject::HostObjectBinding binding{production.id(), asset_ref};
const std::string stored = binding.toString();
const auto reopened = postproject::HostObjectBinding::fromString(stored);
```

Python uses keyed formatting, consistent with its other collection-like reads:

```python
stored = production.host_bindings[asset_id]
binding = production.host_bindings.parse(stored)
assert binding.production_id == production.id
assert binding.object == asset_id
```

Parsing is deliberately strict: versions and object kinds must be known, UUIDs
must use lowercase hyphenated canonical text, and extra fields are rejected. A
future format can therefore be introduced without interpreting ambiguous old
text. The former development-only private-scheme spelling is not accepted.

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
