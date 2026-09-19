# Domain model

- A **Project** is a durable container with a globally unique identity and media
  roots.
- An **Asset** is the logical identity of production media. It is never a path.
- A **Representation** is an original, proxy, optimized, or derived realization
  of an asset. It has one explicit content structure and is not synonymous with
  a file.
- A **ContentStructure** identifies single-resource, image-sequence,
  ordered-part, or package organization.
- A **Resource** is a storage-level component or compact patterned object. File
  facts and resource fingerprints belong here.
- A **Locator** says where or how a resource may be accessed. A resource may
  have multiple locators.
- A **MediaRoot** is an ordered, optional search boundary used by the resolver.
- A **ResourceFingerprint** and a **RepresentationFingerprint** are separate,
  versioned evidence domains. Neither replaces object identity.
- A **Resolution** aggregates required content into online, partial, offline, or
  ambiguous representation availability and retains resource-level evidence.

All mutation occurs inside an explicit transaction. Commit is atomic. Dropping
or rolling back an open transaction makes none of its changes durable, and a
closed transaction rejects further operations.

Filesystem inspection prepares a complete import aggregate before persistence.
Its constructor validates the asset, representation, structure, resource, and
locator relationships. A storage transaction persists the whole aggregate or
none of it.
