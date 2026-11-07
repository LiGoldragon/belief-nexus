# Upgrades

## 0.1.0 to 0.2.0 -- the first balanced-text 0.33 adoption, and a prepared Place at start

Source only. This revision is not deployed and moves no live state by itself.

**Selected cohort.** signal-belief-nexus 0.3.3 (`1a8b`), meta-signal-belief-nexus 0.4.0 (`f4de`),
translator 0.3.2 (`5719`), Signal 10.3.0 (`7afb`), datom-codec 0.33.0 (`eabe`), protos 0.33.0
(`3c7a`) and ethos-zero 19.2.0 (`5a5d`). The stored memory file name and the memory image
encoding are unchanged: no file under `crates/daemon/src` or `crates/core/src` that defines them
is edited.

**Startup change.** translator 0.2.0 moved a Place from `metaflow/NAME` to `nexus/NAME` under the
runtime and state roots. A start now prepares the Place before it opens anything:

- A standard place (state at `<root>/nexus/NAME`) moves a sole legacy `<root>/metaflow/NAME`
  intact to `<root>/nexus/NAME` under the translator per-place exclusion, then creates the
  directories. Both present is refused as `StateConflict` and nothing is changed.
- An explicit place only creates directories; it never looks for legacy state and takes no lock.
- The memory file is then selected inside the same scope: a sole `NAME.sema` file is renamed to the
  current file name; both present is a typed conflict naming both paths. A current name that ends
  in `sema` is refused before any effect, because it would be its own legacy name.
- The launcher validates every input (place name, owner, admission, the five names, the sockets
  against the Place runtime) before it prepares the place or starts a child, prepares the place,
  then starts the Nexus and the translator, and writes the contract envelope and the bootstrap.
  The Nexus validates the same bootstrap with the same validator and prepares the same Place, so a
  direct start cannot open a store the launcher would have moved.
- The Nexus binary reads the 8-byte contract envelope first and refuses a bootstrap of another
  contract before it reads the bootstrap.

**Operator guidance.**

- Conflict, both stores present (`StateConflict` or the memory file conflict): nothing was
  changed. Inspect both, keep the one that holds the data you want, move the other aside by hand,
  and start again. Do not delete either before reading it.
- Failed start after a successful move: the destination is intact. Start again; a repeat moves
  nothing and completes.
- A failed selection or preparation reports its cause; the launcher exits and stops its children.
- The exclusion serializes cooperating starts of a standard place only. An explicit place is not
  serialized, and an alias the process cannot observe is outside it.
- The bootstrap owner and the paired translator are checked against the effective user; that is
  consistency, not authentication of whoever writes the pipe.

How to deploy: repin the user environment's belief-nexus input to this revision, build and switch,
after the lock-service and contract owners agree; none of that is done here.
