# Belief process proof

`run.sh` builds the Nexus, translator and both CLI packages, then runs the
ignored process proof with those exact executables. The proof starts the paired
processes from typed bootstraps and drives configuration and ordinary work only
through the textual meta and ordinary CLIs. It checks Resolve, Register,
Furnish, a quiet live subscription, a Git main change observed by the watch
poller, atomic refurnishing, and durable Memory creation.

It also removes and restores a selected module, verifies the corresponding
published skill path, launches the installed Codex against the furnished
workspace for exact bundle and skill witnesses, then restarts both processes
from the same Memory and proves ordinary service resumes without reconfigure.

The process proof is separate from `nix flake check` because it launches the
built programs and supplies their live process identities.
