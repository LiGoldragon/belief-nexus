#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
pkg="$(nix build --no-link --print-out-paths .#default)"
nix build --no-link .#checks."$(nix eval --impure --raw --expr builtins.currentSystem)".test

base="$(mktemp -d)"
pair_pid=
cleanup() {
  if [[ -n "$pair_pid" ]]; then kill "$pair_pid" 2>/dev/null || true; wait "$pair_pid" 2>/dev/null || true; fi
  rm -rf "$base"
}
trap cleanup EXIT
export XDG_RUNTIME_DIR="$base/run"
runtime="$XDG_RUNTIME_DIR/metaflow/proof"
state="$base/state"
repository="$base/repository"
codex_workspace="$base/codex"
claude_workspace="$base/claude"
mkdir -p "$runtime" "$state" "$repository"/{spirit,knowledge,vision} "$codex_workspace" "$claude_workspace"

printf '%s\n' BUNDLE_DEFAULT_TOKEN > "$repository/spirit/core.md"
printf '%s\n' '---' 'name: knowledge-witness' 'description: Returns the exact skill witness.' '---' '' 'When invoked, answer exactly SKILL_DEFAULT_TOKEN.' > "$repository/knowledge/witness.md"
printf '%s\n' '---' 'placement: system-prompt' '---' BUNDLE_OVERRIDE_TOKEN > "$repository/vision/override.md"
git -C "$repository" init -b main >/dev/null
git -C "$repository" config user.email proof@belief.invalid
git -C "$repository" config user.name proof
git -C "$repository" add .
git -C "$repository" commit -m fixture >/dev/null
git -C "$codex_workspace" init -b main >/dev/null
git -C "$claude_workspace" init -b main >/dev/null

"$pkg/bin/belief-pair" "{ proof «$runtime» «$state» }" >"$base/pair.log" 2>&1 &
pair_pid=$!
for _ in $(seq 1 200); do
  kill -0 "$pair_pid" 2>/dev/null || { cat "$base/pair.log" >&2; exit 1; }
  [[ -S "$runtime/belief-translator.sock" ]] && sleep 0.5 && break
  sleep 0.05
done

meta="$("$pkg/bin/belief-nexus-meta" proof 'Configure.{ None RepositoryPolling.200 }')"
[[ "$meta" == Configured.*Ready* ]]
registered="$("$pkg/bin/belief-nexus" proof "Register.«$repository»")"
[[ "$registered" == Registered.* ]]
modules='[ { Spirit core } { Knowledge witness } { Vision override } ]'
codex_furnished="$("$pkg/bin/belief-nexus" proof "Furnish.{ «$codex_workspace» Codex $modules }")"
claude_furnished="$("$pkg/bin/belief-nexus" proof "Furnish.{ «$claude_workspace» Claude $modules }")"
for result in "$codex_furnished" "$claude_furnished"; do
  [[ "$result" == Furnished.* ]]
  [[ "$result" == *'{ Spirit core }'* && "$result" == *'{ Knowledge witness }'* && "$result" == *'{ Vision override }'* ]]
  [[ "$result" == *'[] }' ]]
done

codex_output="$(codex exec -C "$codex_workspace" 'State both exact bundle tokens. Invoke $knowledge-witness and state its exact skill witness. Output tokens only.' 2>&1)"
claude_output="$(cd "$claude_workspace" && claude --print --dangerously-skip-permissions 'State both exact bundle tokens. Invoke $knowledge-witness and state its exact skill witness. Output tokens only.' 2>&1)"
for output in "$codex_output" "$claude_output"; do
  grep -q BUNDLE_DEFAULT_TOKEN <<<"$output"
  grep -q BUNDLE_OVERRIDE_TOKEN <<<"$output"
  grep -q SKILL_DEFAULT_TOKEN <<<"$output"
done

printf '%s\n' "$meta" "$registered" "$codex_furnished" "$claude_furnished"
printf 'Codex: BUNDLE_DEFAULT_TOKEN BUNDLE_OVERRIDE_TOKEN SKILL_DEFAULT_TOKEN\n'
printf 'Claude: BUNDLE_DEFAULT_TOKEN BUNDLE_OVERRIDE_TOKEN SKILL_DEFAULT_TOKEN\n'
