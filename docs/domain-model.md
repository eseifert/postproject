# Domain model

- A **Project** is a durable container with a globally unique identity and media
  roots.
- An **Asset** is the logical identity of production media. It is never a path.
- A **Representation** is an original, proxy, optimized, or derived encoding of
  an asset.
- A **Location** says where a representation may physically exist. A
  representation may have multiple locations.
- A **MediaRoot** is an ordered, optional search boundary used by the resolver.
- A **Fingerprint** is versioned evidence derived from file facts and selected
  bytes. Partial fingerprints are not collision-proof.
- A **Resolution** is a state plus inspectable evidence and ordered candidates;
  ambiguity is an explicit result, never an automatic choice.

All mutation occurs inside an explicit transaction. Commit is atomic. Dropping
or rolling back an open transaction makes none of its changes durable, and a
closed transaction rejects further operations.

Filesystem inspection produces an `OriginalMediaImport` aggregate before
persistence. Its constructor guarantees that the original representation belongs
to the asset and the location belongs to that representation. A storage
transaction persists the whole aggregate or none of it.
