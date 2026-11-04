# belief-nexus

Belief Nexus version 1 registers belief repositories, resolves typed module
names and atomically furnishes Codex workspaces. Its ordinary and meta wire
contracts live in the adjacent `signal-belief-nexus` and
`meta-signal-belief-nexus` repositories.

The component is split into core, daemon, translator and emission crates. The
Nexus accepts typed Signal only. `belief-translator` owns text and external
effect boundaries: it turns one datom into an authenticated typed call, and it
executes typed Git snapshot and filesystem publication emissions from the
paired Nexus. `belief-nexus` and `belief-nexus-meta` each take a translator
socket followed by one inline datom.

`emission.ethos` is temporarily authored with a `Signal` root because Ethos
Zero does not yet generate an `Emission` root. It nevertheless declares only
the Nexus-to-translator effect contract.

## Nix

The flake exposes `translator`, `cli`, and `meta-cli` packages. `nix flake
check` builds the workspace tests and verifies that the generated prefix of the
emission Rust projection exactly matches its Ethos declaration. The root
`Cargo.lock` is committed deliberately so the workspace and the Nix proof use
one dependency graph.

The end-to-end proof entry point is `proof/run.sh`. It is a scaffold until the
daemon-owned fixture supplies typed bootstraps and real temporary Git and Codex
workspaces; the scaffold itself makes no proof claim.
