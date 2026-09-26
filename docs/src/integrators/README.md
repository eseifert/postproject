# Integrator guide

This section is for developers adding PostProject to a host application, plugin, service, or pipeline tool.

The fastest successful integration is usually **smaller than the whole PostProject feature set**. Begin with durable identity and media resolution, prove that the host can reopen the same production objects, then add richer knowledge only when the application has a use for it.

## Recommended integration path

### 1. Install and open a production

Choose a public surface:

- **C** — the native ABI and lowest common denominator;
- **C++17** — convenience wrapper over the native API;
- **Python** — binding over the C ABI;
- **CLI** — useful for scripting, diagnostics, fixtures, and integration tests.

Start with {doc}`installing-a-release`, then choose {doc}`c-quickstart`, {doc}`cpp-quickstart`, or {doc}`python`.

### 2. Create or open media identities

Follow {doc}`first-production`. Store the returned PostProject identity in the host's own project/object model where appropriate.

Do not substitute a filesystem path for that identity. The entire portability model depends on those being different concepts.

### 3. Make media resolution part of host behavior

Implement {doc}`media-resolution` before building elaborate metadata features. A useful integration should survive media moving to a different mount or workstation.

The host should surface ambiguity rather than silently accepting the first candidate.

### 4. Support the media structures your application actually uses

If the host handles VFX or camera media, continue with {doc}`compound-media` so sequences, spans, and packages remain one representation instead of becoming one logical asset per file.

### 5. Add production knowledge incrementally

Use the pieces that solve real host problems:

- {doc}`external-identifiers` for identifiers owned by other systems;
- {doc}`metadata-vocabularies` for structured metadata;
- {doc}`provenance` for activity history;
- {doc}`revision-feed` for reacting to changes;
- {doc}`host-object-bindings` for durable links back to host objects;
- {doc}`bounded-queries` for scalable traversal;
- {doc}`jobs-and-workers` when the host coordinates durable production work;
- {doc}`reference-executor` as an example of the execution boundary, not as a requirement.

## Integration rules that matter early

A few rules prevent most architectural mistakes:

1. **PostProject identity is internal production identity.** Do not pretend a PostProject UUID is a UMID, EIDR, camera ID, or another external scheme.
2. **Preserve external identifiers exactly.** Store their scheme and value rather than translating them into the internal identity space.
3. **Never silently choose an ambiguous relink candidate.** Ambiguity is a meaningful result.
4. **Perform mutations in explicit transactions.** A transaction groups one semantic production change and its revision history.
5. **Advance a revision cursor only after a complete revision has been processed.** This keeps consumers restartable.
6. **Release owned C handles.** Treat the ownership rules in the native API as part of the contract.
7. **Do not infer relationships that were not recorded.** An activity connecting media does not automatically mean “revision,” “variant,” or “alternative.”

## Separate semantic documentation from API lookup

Use the concept pages when deciding **what an operation means**. Use generated API reference when deciding **how to call it**.

For example:

- {doc}`../concepts/assets-representations-locators` explains the representation model.
- {doc}`compound-media` shows integration workflow.
- the generated C/C++/Python reference gives exact signatures and ownership details.

This separation keeps the conceptual contract readable while allowing API reference to remain precise.

## Rust is not required for downstream integration

PostProject is implemented in Rust, but an installed native consumer should not need Cargo or Rust tooling. The public native boundary is the C ABI, with C++ and Python layers built on top of it.

If you are changing PostProject itself rather than integrating it, use {doc}`../contributors/README`.
