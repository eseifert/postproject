# Identity

PostProject object IDs answer “which object in this project model is this?” A
project, logical asset, concrete representation, location, activity, and
revision each have a distinct ID type. Matching UUID bytes do not make two
different object types interchangeable.

An industry, registry, camera, vendor, or application identifier answers a
different question. A UMID, EIDR, ISAN, camera serial, or application ID is
stored as an [external identifier](external-identifiers.md); it never replaces
an `AssetId` or `RepresentationId`.

This separation lets one object carry several identifiers without pretending
that any single external scheme defines PostProject's storage identity.
