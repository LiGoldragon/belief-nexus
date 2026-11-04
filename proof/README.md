# Belief proof harness

`run.sh` is the binary boundary for the version-one proof. It deliberately
requires the daemon, translator, ordinary CLI and meta CLI as explicit paths.
The daemon integration supplies the typed bootstraps and the temporary Git and
Codex workspaces; this scaffold makes no success claim by itself.
