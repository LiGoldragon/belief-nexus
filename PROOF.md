# Belief Nexus version 1 proof

Run from the repository root:

```sh
nix flake check -L
./proof/run.sh
```

`proof/run.sh` builds `.#nexus`, `.#translator`, `.#cli`, and `.#meta-cli` and
runs their actual executables. The process proof observes:

- typed bootstrap and empty-Memory, one-field-at-a-time configuration reaching
  readiness through the meta CLI;
- authenticated ordinary Register, Resolve, Furnish, and Subscribe calls
  through the ordinary CLI;
- explicit found and missing module results;
- an initial subscription state, a quiet open connection, and pushed Changed
  events;
- immutable committed-main snapshots, atomic refurnishing, and removal and
  restoration of a selected module;
- a real installed Codex start, `codex --no-daemon exec -C <workspace>`, whose
  last message contains the exact system-prompt token `BUNDLE_WITNESS_1` and
  the exact invoked-skill token `SKILL_WITNESS_1`;
- process restart from the same durable Memory, after which the translator
  restores its authoritative configuration and an ordinary Resolve succeeds
  without reconfiguration.

The ordinary unit and process tests separately derive caller identity from
`SO_PEERCRED` and the live `/proc/<pid>/exe`, reject self-asserted callers, and
treat the contract digest as compatibility rather than authority. Filesystem
tests cover pre-pointer failure, owned-pointer and manifest inspection,
foreign current paths, settings conflicts and malformed settings, unrelated
settings preservation, and exact cleanup targets.
