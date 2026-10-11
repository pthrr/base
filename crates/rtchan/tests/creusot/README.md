# Creusot proof boundary

`task test:creusot` verifies the production source files through a harness
that includes them directly. Ghost fields are present only under `cfg(creusot)`;
the task also runs Creusot's erasure check. The proof shell is available on
x86_64 Linux.

| Rust code | Proven against the stated contracts |
| --- | --- |
| [Channel](../../src/channel.rs) | Both endpoints start with empty histories and the requested capacity. Successful sends append their value; failed sends return it. The producer's consumed history is a prefix of its accepted history; the consumer's delivered history is a prefix of its observed FIFO. If that FIFO was nonempty before `try_recv`, the call returns its head. `len()` equals the size of the FIFO at its atomic observation. |
| [Event data input](../../src/event_data_input.rs) | Capacity stays at least `Queue_Size`; the `DropOldest` allocation fits `max_sends` sends after fewer than `Queue_Size` queued items. Both policies read length at input time. If the observed FIFO is `R` and `d` older values are trimmed, `dispatch()` freezes the head of `R[d..]`, or `None` when empty. At freeze completion, the remaining **observed** FIFO is exactly the unconsumed suffix from the length read followed by values newly observed during the freeze. `has_pending()` reports whether its length observation was positive. Completing a guard returns its value; dropping it clears the active flag. |
| [Event input](../../src/event_input.rs) | The unit-valued wrapper preserves the event data freeze and delivered-count contracts. |
| [Data input](../../src/data_input.rs) | Construction capacity; the inner queue size remains one. `Updated` holds exactly when the input-time length observation was positive; that dispatch selects its newest observed value. An empty observation leaves the retained value unchanged and consumes nothing. |

The [external `rtrb` specifications](src/rtrb_contract.rs) and the one-call
`observe_slots` wrapper in [channel.rs](../../src/channel.rs) are the trusted
boundary. They assume bounded FIFO histories, exact `Full` and `Empty`
observations, and an atomic tail read that updates the receiver's observed
history. Two length reads can therefore differ after a send. These are
assumptions about `rtrb`, not proofs of its code.

The histories agree at construction because all four are empty. There is
**no proved preservation of a shared-state relation** between the producer's
accepted history and the consumer's offered history after separate endpoint
operations. A former `observe_pair` axiom
claimed such a relation. A consistency probe showed that it let Creusot prove
`false` after a send and receive; the axiom and its dependent proof were
removed. With the axiom removed, the same false assertion is unproved.
The contradiction probe also remains unproved after strengthening the
producer's local prefix contract. This is a regression check for an obvious
inconsistency, not a proof that all trusted assumptions are consistent.
An opposite probe also remains unproved: after a successful send of `7`, the
current contracts cannot establish that the next receive returns `7`, even
when both endpoints are used sequentially. This identifies the missing
producer-to-consumer link precisely.
Closing this gap requires a sound shared resource invariant or a verified
`rtrb` model that links the endpoint histories.
An immutable value sequence alone is insufficient: the current consumer
contract can observe a value before any send, then conflict with a later send.
At each queue linearization point, successful pushes `A` and pops `D` must
satisfy `|D| <= |A|`, `D = A[0..|D|]`, and `Q = A[|D|..]` with
`|Q| <= capacity`. A receiver observation `O` must be a published prefix:
`D <= O <= A` in prefix order at that point.

The application must establish the `max_sends` arrival bound and align the
length read inside `dispatch()` with the declared input time. The proof does
not establish either application fact, the behavior of `is_abandoned()` after
peer drop, or bounds on user-defined destructors. The `has_pending()` result
describes its length observation; a later send can make that observation stale. The
[mCRL2 fixtures](../README.md) check finite models; a universal correspondence
between those models and concurrent Rust executions has not been machine
checked. Thus a green Creusot run proves the listed local contracts, not
complete Rust-to-mCRL2 refinement.
