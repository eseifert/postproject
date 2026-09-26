#!/usr/bin/env bash
# Runs the CLI listings of the production lifecycle guide.
#
# Each "[name]" ... "[/name]" region is included verbatim by the documentation
# build, so keep regions self-contained and readable. Usage:
#   PATH=/opt/postproject/bin:$PATH lifecycle.sh WORK_DIRECTORY
# The work directory is prepared by prepare-workdir.cmake. Requires jq.
#
# Every CLI command runs in its own transaction and commits on success, so
# this surface has no explicit transaction listing.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: lifecycle.sh WORK_DIRECTORY" >&2
  exit 2
fi
cd "$1"

PRODUCTION_ID=$(postproject --json init lifecycle.pproj --name "Documentary" |
  jq -r .id)
ASSET_ID=$(postproject --json media add lifecycle.pproj rushes/A001.mov |
  jq -r .asset_id)

# [open-production]
# Each command opens the production file, reads it, and closes it again.
postproject --json media list lifecycle.pproj > assets.json
jq -r '.[] | "\(.id) \(.representation_count) representation(s)"' assets.json
if jq -e --arg id "$ASSET_ID" 'any(.[]; .id == $id)' assets.json >/dev/null; then
  echo "asset $ASSET_ID exists"
fi
postproject --json revisions latest lifecycle.pproj | jq '{id, sequence, message}'
# [/open-production]

test "$(jq length assets.json)" = 1
test "$(postproject --json revisions latest lifecycle.pproj | jq -r .sequence)" = 1
test -n "$PRODUCTION_ID"

# [error-handling]
# Failures exit with a non-zero status and describe the error on stderr.
if ! postproject media list missing.pproj 2> error.txt; then
  echo "open failed: $(cat error.txt)"
fi
# [/error-handling]

grep -q "does not exist" error.txt
if postproject media list missing.pproj 2>/dev/null; then
  echo "opening a missing production unexpectedly succeeded" >&2
  exit 1
fi
test ! -e missing.pproj
