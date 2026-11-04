# belief-nexus

Fresh version-1 Belief Nexus scaffolding. This repository is a public proof-of-concept workspace and currently contains no functional Nexus implementation.

The intended component boundary is separate core memory/daemon, translator and client libraries/binaries, and an Emission Signal stand-in. The Signal root in `emission.ethos` is temporary: it does not modify ethos-zero and is not generated yet.

Belief will eventually register module sources, resolve typed module names, and furnish workspaces with generated bundles. File-backed Belief is the first mock boundary. Placement is derived from the source-folder Kind: Spirit and Intent modules enter the system-prompt bundle; every other Kind enters the skills directory. Watch/refurnish and perma-flow refresh remain deferred.
