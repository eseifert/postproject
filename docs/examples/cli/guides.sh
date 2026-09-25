#!/usr/bin/env bash
# Runs every CLI listing included in the PostProject integrator guides.
#
# Each "[name]" ... "[/name]" region is included verbatim by the documentation
# build, so keep regions self-contained and readable. Usage:
#   PATH=/opt/postproject/bin:$PATH guides.sh WORK_DIRECTORY
# The work directory is prepared by prepare-workdir.cmake. Requires jq.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: guides.sh WORK_DIRECTORY" >&2
  exit 2
fi
cd "$1"

# [create-production]
postproject init production.pproj --name "Documentary"
ASSET_ID=$(postproject --json media add production.pproj rushes/A001.mov \
  --name "Camera A" | jq -r .asset_id)
postproject media show production.pproj "$ASSET_ID"
# [/create-production]

ORIGINAL_ID=$(postproject --json media show production.pproj "$ASSET_ID" |
  jq -r '.representations[0].id')

# [external-identifiers]
postproject identifier add production.pproj asset "$ASSET_ID" \
  com.example.camera.serial A-0007
postproject identifier list production.pproj asset "$ASSET_ID"
postproject identifier find production.pproj com.example.camera.serial A-0007
# [/external-identifiers]

# [metadata]
IPTC_VMHUB=https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json
postproject metadata add-text production.pproj asset "$ASSET_ID" \
  "$IPTC_VMHUB" title "Interview" --language en-US
postproject metadata list production.pproj asset "$ASSET_ID"
postproject metadata find production.pproj "$IPTC_VMHUB" title
# [/metadata]

# [media-root]
postproject root add production.pproj rushes --label "Camera originals"
# [/media-root]

mv rushes/A001.mov moved/A001.mov

# [resolve-asset]
postproject --json media resolve production.pproj "$ASSET_ID" \
  --root-map rushes="$PWD/moved" > resolution.json
jq -r '.resolutions[] | .availability' resolution.json
jq -r '.resolutions[].resources[].candidates[].uri' resolution.json
# [/resolve-asset]

# [confirm-locator]
# Confirm only a candidate that a person or policy selected; never pick one of
# several plausible candidates automatically.
CANDIDATE=$(jq -r '.resolutions[0].resources[0].candidates[0].uri' resolution.json)
postproject media resolve production.pproj "$ASSET_ID" \
  --root-map rushes="$PWD/moved" --confirm "$CANDIDATE"
# [/confirm-locator]

# [image-sequence]
cat > shot010.json <<'EOF'
{
  "structure": "image_sequence",
  "directory": "renders/shot010",
  "prefix": "shot010.",
  "suffix": ".exr",
  "padding": 4,
  "start": 1001,
  "end": 1004,
  "step": 1,
  "rate_numerator": 24000,
  "rate_denominator": 1001,
  "missing_frames": [1003]
}
EOF
SEQUENCE_ID=$(postproject --json representation add production.pproj \
  "$ASSET_ID" derived shot010.json | jq -r .representation_id)
postproject media show production.pproj "$ASSET_ID"
# [/image-sequence]

# [provenance]
postproject activity add production.pproj org.postproject:render \
  --input "$ORIGINAL_ID=org.postproject:primary" \
  --output "$SEQUENCE_ID" \
  --tool-name "Example Renderer" --tool-version 2.1 \
  --tool-uri https://example.com/renderer
postproject activity producing production.pproj "$SEQUENCE_ID"
postproject activity ancestors production.pproj "$SEQUENCE_ID"
# [/provenance]

test "$(postproject --json activity ancestors production.pproj "$SEQUENCE_ID" |
  jq -r '.items[0].representation_id')" = "$ORIGINAL_ID"

# [artifact-knowledge]
postproject --json artifact evaluate production.pproj "$SEQUENCE_ID" \
  --max-depth 64 --max-representations 1000 |
  jq '{state, reasons, visited_representations, truncated}'
postproject --json artifact reproducibility production.pproj "$SEQUENCE_ID" |
  jq '{reproducible, issues}'
# [/artifact-knowledge]

# [dependency-queries]
cat > dependency.json <<EOF
[
  {
    "kind": "org.example:character-reference",
    "target": {"kind": "asset", "id": "$ASSET_ID"},
    "resolved_representation_id": "$ORIGINAL_ID",
    "required": true,
    "authored_reference": "characters/lead.usd"
  }
]
EOF
postproject dependency record production.pproj "$SEQUENCE_ID" dependency.json
postproject --json dependency dependencies production.pproj "$SEQUENCE_ID" \
  --max-depth 4 --max-representations 1000 --limit 100 |
  jq '.items[] | {target, depth}'
postproject dependency dependents production.pproj asset "$ASSET_ID" \
  --max-depth 4 --max-representations 1000 --limit 100
# [/dependency-queries]

# [job-query-pages]
for _ in 1 2; do
  postproject job request production.pproj org.example:generate-proxy \
    "$ASSET_ID" proxy --input "$ORIGINAL_ID"
done
CURSOR=
while :; do
  ARGS=(--json job list production.pproj --state requested \
    --kind org.example:generate-proxy --limit 1)
  [[ -n "$CURSOR" ]] && ARGS+=(--cursor "$CURSOR")
  PAGE=$(postproject "${ARGS[@]}")
  jq '.items[] | {id, kind, state}' <<<"$PAGE"
  CURSOR=$(jq -r '.next_cursor // empty' <<<"$PAGE")
  [[ -z "$CURSOR" ]] && break
done
# [/job-query-pages]

# [reference-executor]
mkdir -p proxies
postproject root add production.pproj proxies --label "Generated proxies"
cat > ffmpeg-fake <<'EOF'
#!/bin/sh
if [ "$1" = "-version" ]; then
  echo "ffmpeg version guide-fake"
  exit 0
fi
for output do :; done
printf 'proxy media' > "$output"
EOF
chmod +x ffmpeg-fake
postproject job request production.pproj org.postproject:generate-proxy \
  "$ASSET_ID" proxy --input "$ORIGINAL_ID" --target-root proxies \
  --profile proxy-720p
postproject job run production.pproj --once \
  --root-map proxies="$PWD/proxies" --ffmpeg "$PWD/ffmpeg-fake"
# [/reference-executor]

# [revision-feed]
CURSOR=0
while :; do
  PAGE=$(postproject --json revisions since production.pproj \
    --after "$CURSOR" --limit 100)
  for REVISION_ID in $(jq -r '.[].id' <<<"$PAGE"); do
    postproject --json revisions events production.pproj "$REVISION_ID"
    # Persist the cursor only after the whole revision is processed.
    CURSOR=$(jq -r --arg id "$REVISION_ID" '.[] | select(.id == $id) | .sequence' <<<"$PAGE")
  done
  [[ $(jq length <<<"$PAGE") -lt 100 ]] && break
done
# [/revision-feed]

test "$CURSOR" = "$(postproject --json revisions latest production.pproj | jq -r .sequence)"
