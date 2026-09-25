# Identity

PostProject object IDs answer “which object in this production model is this?” A
production, logical asset, concrete representation, storage resource, locator,
activity, job, and revision each have a distinct ID type. Matching UUID bytes
do not make two different object types interchangeable. A job claim ID is a
short-lived capability for one active claim, not another durable object.

An industry, registry, camera, vendor, or application identifier answers a
different question. A UMID, EIDR, ISAN, camera serial, or application ID is
stored as an [external identifier](external-identifiers.md); it never replaces
an `AssetId`, `RepresentationId`, or `ResourceId`.

This separation lets one object carry several identifiers without pretending
that any single external scheme defines PostProject's storage identity.

## Across public surfaces

The following example serializes a typed host-object binding and parses it back
without collapsing the production or object kind into raw UUID text:

```{code-variants} host-binding
```
