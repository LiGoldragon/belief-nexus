# belief-nexus

Belief Nexus version 1 registers belief repositories, resolves typed module
names, and atomically furnishes Codex and Claude workspaces. Its ordinary and
meta contracts live in `signal-belief-nexus` and
`meta-signal-belief-nexus`.

A registered repository is read at its committed `refs/heads/main`. `Resolve`
returns found and missing names; missing is partial success. `Furnish` returns
`FurnishedPaths { SystemPromptPath, SkillsPath }`. A module can declare exact
frontmatter `placement: system-prompt` or `placement: skills`; otherwise the
stored Kind default applies. The packaged deployment defaults Spirit and
Intent to the bundle and Vision, Knowledge, and Role to skills. Invalid,
duplicate, or non-scalar placement frontmatter is a typed refusal.

Publication writes an immutable generation and swaps one owned `current`
pointer. Codex receives an absolute stable `model_instructions_file` and the
owned `.agents/skills` link. Claude receives owned root `CLAUDE.md` and
`.claude/skills` links. Foreign destinations are refused before the swap.
Subscriptions open with current state and then receive changes; committed-main
changes automatically refurnish affected workspaces, including module removal
and restoration.

The Nexus accepts binary Signal only, on the exchange layer every Nexus
shares (`nexus-serving`): each socket greets with its call contract's digest,
and every exchange is answered once, or for a Subscribe with the frames of a
`signal::Stream` (`Opened`, `Changed`, `Lagged`), and then ended. The paired
translator, on the serving every translator shares (`translator-serving`),
owns datom text, Git snapshots, and filesystem publication. Public commands
are:

```sh
belief-pair '{ <place-name> «<runtime-directory>» «<state-directory>» }'
belief-nexus '<ordinary datom>'
belief-nexus-meta '<meta datom>'
```

`belief-pair` takes exactly one typed `Place` and runs the pair launcher
every translator shares; its Nix package supplies the Nexus, the translator
with its deployment (`crates/translator/deployment.datom`), the admission and
the text budget. Each CLI takes its datom alone: its place is the one
`$XDG_CONFIG_HOME/nexus/belief.datom` (`~/.config/nexus/belief.datom`) names,
`{ <place-name> }`, resolved under `$XDG_RUNTIME_DIR/nexus/`. Socket names are
`belief.sock`, `belief-meta.sock`, `belief-translator.sock`, and
`belief-meta-translator.sock`; Memory is `belief.memory` under State.

Text calls are owned by `Owner.Login(UserId)`, so separate CLI processes of the
same login share repositories and workspaces. An authenticated direct signal
peer is owned by its configured `Owner.Peer(PeerId)` after its declared Caller
matches kernel-observed PID and UID. Meta calls require the paired translator
transport and the login the Nexus runs as. Configuration changes one field
atomically: `None` is an authorized unconditional set; `Some(revision)` is
compare-and-set. `Configured` returns the resulting coherent revision. At its
first start the translator applies its deployment through the Nexus, from
revision 0; a Nexus that already holds it is left as it is.

`emission.ethos` temporarily uses a `Signal` root because Ethos Zero does not
yet emit an `Emission` root; it declares only the Nexus-to-translator effect
contract.

## Nix and proof

The default flake package contains the Nexus, translator, pair, ordinary CLI,
and meta CLI. Individual selectors are `nexus`, `translator`, `pair`, `cli`,
and `meta-cli`. The committed root `Cargo.lock` fixes the graph.

```sh
nix flake check -L
./proof/run.sh
```

The proof uses the packaged pair and public CLIs, furnishes separate Codex and
Claude workspaces, and starts the installed real harnesses with existing login
credentials. See `PROOF.md`.
