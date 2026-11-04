#!/usr/bin/env bash
set -euo pipefail

: "${BELIEF_NEXUS:?path to the built daemon binary}"
: "${BELIEF_TRANSLATOR:?path to the built translator binary}"
: "${BELIEF_CLI:?path to the built ordinary CLI}"
: "${BELIEF_META_CLI:?path to the built meta CLI}"

for executable in "$BELIEF_NEXUS" "$BELIEF_TRANSLATOR" "$BELIEF_CLI" "$BELIEF_META_CLI"; do
  test -x "$executable"
done

echo "Belief proof binaries are present; daemon-owned end-to-end fixture plugs in here."
