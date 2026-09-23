# Portable production workflow

This walkthrough joins the Iteration 3 features into one operator story. Use
the `postproject` executable from a [native release archive](../integrators/installing-a-release.md);
none of these steps invokes Cargo.

## Import real and compound media

Create the production, register a logical search root, and import a movie with
optional technical inspection:

```sh
postproject init documentary.pproj --name "Documentary"
postproject root add documentary.pproj originals --label "Camera originals"
postproject media add documentary.pproj /media/A001.mov --name "A001" --inspect
```

`--inspect` invokes `ffprobe` as a bounded subprocess and records normalized
technical values through the metadata vocabulary. Missing or rejected
inspection is reported but does not cancel the import.

A directory identifying one numbered group becomes one image-sequence
representation. Its rate stays explicit. A directory containing the checked
AVCHD shape (`PRIVATE/AVCHD/BDMV`) becomes one package whose stream files are
required essence and whose clip information, playlists, and navigation files
retain their roles:

```sh
postproject media add documentary.pproj /media/plates/shot010 \
  --name "Shot 010 plates" --sequence-rate 24/1
postproject media add documentary.pproj /media/CARD_001 --name "Card 001"
```

The import result prints each asset ID. Keep those IDs for resolution calls or
retrieve them later with `postproject media list documentary.pproj`.

## Move the storage

Copy `documentary.pproj` and the media to another machine. The production keeps
the logical root name, while the new mount is supplied per call:

```sh
postproject media resolve documentary.pproj MOV_ASSET_ID \
  --root-map originals=/mnt/documentary
```

An unmapped root is reported as `unmapped`; an unreadable mapping is
`unavailable`. Other usable roots are still searched. If a relocated candidate
is unique, pass its returned URI through `--confirm` to record the new locator.
PostProject never confirms one of several plausible candidates automatically.

## Detect damage and inspect the inventory

Delete one frame from the copied EXR sequence, then request the expensive
verification tier and a non-mutating inventory:

```sh
postproject media resolve documentary.pproj SEQUENCE_ASSET_ID \
  --root-map originals=/mnt/documentary --verify --json
postproject media inventory documentary.pproj \
  --root-map originals=/mnt/documentary \
  --cache /var/tmp/documentary-inventory.json --json
```

The sequence resolves as `partial` and names the absent frame. Verification
also detects a file replaced in place by recomputing stored fingerprints.
Inventory reports known online, partial, missing, new, changed, duplicate, and
ambiguous media without mutating the production. Its sidecar cache is
machine-local and disposable; removing it changes scan cost, not results.

## Resolve an editorial reference

The maintained [OpenAssetIO Manager](https://github.com/eseifert/postproject-openassetio-manager)
accepts the same versioned representation bindings exposed by PostProject. The
[OTIO demonstration](https://github.com/eseifert/postproject-otio-demo) stores
one in an ordinary `ExternalReference` and lets the upstream OpenAssetIO media
linker resolve it to locatable content. Rational clip time remains owned by
OTIO, and an image sequence remains one PostProject representation.

