#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

package_bin() {
  local selector="$1" binary="$2" output
  output="$(nix build --no-link --print-out-paths ".#$selector")"
  printf '%s/bin/%s' "$output" "$binary"
}

export BELIEF_NEXUS="${BELIEF_NEXUS:-$(package_bin nexus belief-nexus-daemon)}"
export BELIEF_TRANSLATOR="${BELIEF_TRANSLATOR:-$(package_bin translator belief-translator)}"
export BELIEF_CLI="${BELIEF_CLI:-$(package_bin cli belief-nexus)}"
export BELIEF_META_CLI="${BELIEF_META_CLI:-$(package_bin meta-cli belief-nexus-meta)}"
export BELIEF_GIT="${BELIEF_GIT:-$(command -v git)}"
export BELIEF_CODEX="${BELIEF_CODEX:-$(command -v codex)}"

for executable in "$BELIEF_NEXUS" "$BELIEF_TRANSLATOR" "$BELIEF_CLI" "$BELIEF_META_CLI" "$BELIEF_GIT" "$BELIEF_CODEX"; do
  test -x "$executable"
done

nix develop -c cargo test -p belief-nexus-daemon --test e2e \
  translated_clis_subscription_watch_and_restore -- --ignored --exact --nocapture
