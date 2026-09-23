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
  jq -r '.[0].representation_id')" = "$ORIGINAL_ID"

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
