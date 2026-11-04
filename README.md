# belief-nexus

Belief Nexus version 1 registers belief repositories, resolves typed module
names and atomically furnishes Codex workspaces. Its ordinary and meta wire
contracts live in the adjacent `signal-belief-nexus` and
`meta-signal-belief-nexus` repositories.

Belief is the rewrite of Curriculum: the skill generator named for what a
model believes when it starts. A registered repository is observed at its
committed `refs/heads/main`. `Resolve` returns found and missing module names;
missing is partial success. `Furnish` publishes spirit and intent into the
system-prompt bundle and every other kind as a skill. A subscription first
receives the complete current belief state and then each change. Repository
changes, including removed and restored modules, automatically refurnish each
affected workspace.

The component is split into core, daemon, translator and emission crates. The
Nexus accepts typed Signal only. `belief-translator` owns text and external
effect boundaries: it turns one datom into an authenticated typed call, and it
executes typed Git snapshot and filesystem publication emissions from the
paired Nexus. `belief-nexus` and `belief-nexus-meta` each take a translator
socket followed by one inline datom in the present bootstrap stand-in. Their
final public shape is `belief-nexus <metaflow> '<datom>'` (and the same for
`belief-nexus-meta`); resolving the metaflow through the fixed instance
registry is waiting on the shared registry contract rather than accepting a
registry path or socket as public CLI configuration.

Each successful furnish reports stable `Placement` paths. For Codex these are
the absolute `.belief/current/instructions.md` path and `.agents/skills`.
Publication writes the absolute instructions path to
`.codex/config.toml`, preserves unrelated settings, and maintains the skills
path as an owned stable link through the atomic current generation. A running
Codex started with `codex exec -C <workspace>` therefore reads the bundle and
discovers the skills without a separate credential home.

Configuration is set one field at a time and stored with one coherent
revision. On start the paired translator restores the authoritative durable
snapshot. Meta configuration stays available while the configuration is
partial; ordinary admission waits until it is ready. Every emission attempt
carries the immutable translator snapshot and revision admitted for that
attempt, so a later reconfiguration cannot mix into work already admitted.

`emission.ethos` is temporarily authored with a `Signal` root because Ethos
Zero does not yet generate an `Emission` root. It nevertheless declares only
the Nexus-to-translator effect contract.

## Nix

The flake's default is the `nexus` package and it also exposes `translator`,
`cli`, and `meta-cli`. `nix flake
check` builds the workspace tests and verifies that the generated prefix of the
emission Rust projection exactly matches its Ethos declaration. The root
`Cargo.lock` is committed deliberately so the workspace and the Nix proof use
one dependency graph.

The end-to-end proof entry point is `proof/run.sh`. It builds and launches the
four packaged programs, supplies typed bootstraps, configures a new durable
Memory through the meta CLI, and drives a temporary Git repository and Codex
workspace through the ordinary CLI. See `PROOF.md` for its witnesses.
