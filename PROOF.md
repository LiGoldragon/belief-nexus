# Belief Nexus version 1 proof

Run from the repository root with existing Codex and Claude login credentials:

```sh
nix flake check -L
./proof/run.sh
```

The script builds the default package and the Nix test check, starts
`belief-pair` from a typed Place, and calls the public meta and ordinary CLIs
from separate processes. Its fixture has:

- Spirit without placement frontmatter, exercising the stored SystemPrompt
  default;
- Knowledge without placement frontmatter, exercising the stored Skills
  default;
- Vision with `placement: system-prompt`, exercising an explicit override of
  its stored Skills default.

It registers the committed-main repository and furnishes separate Codex and
Claude workspaces. Each `Furnished` reply carries stable SystemPromptPath and
SkillsPath, all three found names, and an empty missing vector. It then starts:

```sh
codex exec -C <codex-workspace> '<witness request>'
(cd <claude-workspace> && claude --print --dangerously-skip-permissions '<witness request>')
```

Both real harnesses must return all exact witnesses:
`BUNDLE_DEFAULT_TOKEN`, `BUNDLE_OVERRIDE_TOKEN`, and `SKILL_DEFAULT_TOKEN`.
No credential is copied or replaced.

The Nix check contains the independent typed and process evidence: configured
direct-peer success and unconfigured refusal from kernel Caller facts; PeerId
owner isolation; Login ownership across CLI PIDs; strict placement parsing;
found/missing resolution; subscription initial state and capacity; quiet watch;
automatic removed/restored refurnish; ambiguity preserving the current
publication; pending/recovery durability; atomic pointer publication; settings
preservation and conflict refusal; exact cleanup; and committed-main snapshots
that ignore worktree changes.

Observed packaged first-launch witness at belief commit `9a6ead4`:

```text
belief-nexus-meta proof 'Configure.{ None RepositoryPolling.200 }'
Configured.{ RepositoryPolling.200 Nexus 24 Ready }

belief-nexus proof 'Resolve.[]'
Resolved.{ [] [] }
```
