# Creusot proof boundary

`task test:creusot` checks the production channel and all three port source files with
Creusot. The harness includes those files directly, so it verifies the same
method bodies that `rtchan` compiles. The Nix `proof` shell is available on
x86_64 Linux.

The stated contracts cover channel result conversion, positive construction
capacity, `DropOldest` trim count, latest-value update, frozen-value access,
and construction of an inactive event data input. Other method bodies are checked
for safety under their preconditions; they do not acquire an unstated FIFO or
AADL refinement guarantee.

The [trusted `rtrb` contract](src/rtrb_contract.rs) now records successful
send and receive histories. It assumes that a successful pop returns the next
offered value, that a nonempty input cannot report empty, that a producer with
room cannot report full, and that the two endpoint histories form one bounded
FIFO when both endpoints are observed without an operation in progress. It
also bounds `slots()` by capacity. The outcome postconditions are written to
allow a peer operation during a call; for example, `Empty` does not assert
that the queue is still empty at return.

Creusot proves that [`channel`'s production methods](../../src/channel.rs)
preserve the accepted/delivered histories and convert `rtrb` outcomes without
changing their values. A proof function checks that two accepted messages are
received in order while both endpoints are held. The port modules still have
the local contracts listed above; this does not establish their full AADL
refinement. The external specifications and the quiescent FIFO relation are
**assumptions about `rtrb`**, not proofs of its implementation. The
[mapping argument](../../MAP.md#verification-boundary) and
[finite model fixtures](../README.md) remain separate; their correspondence
to arbitrary Rust executions is not machine checked.
