# AADL port semantics and the rtchan mapping

This document maps three restricted AADL input-port kinds to Rust and mCRL2.
The AADL reference is [AS5506D](https://www.sae.org/standards/content/as5506d/);
the [standard property declarations](https://github.com/OpenAADL/ocarina/blob/master/resources/AADLv2/communication_properties.aadl)
name queue and overflow settings, and the [HAMR semantics note](https://hamr.sireum.org/hamr-doc/hamr-semantics-concepts/)
describes dispatch-time input freezing. The rules below state the exact
profile checked in this repository.

## Profile

| AADL property | Required value | Rust input |
| --- | --- | --- |
| Port kind | `in event port` | `EventInput<P>`; occurrence payload is `()` |
| Port kind | `in event data port` | `EventDataInput<T, P>` |
| Port kind | `in data port` on a sampled connection | `DataInput<T>` |
| Incoming connection | Exactly one source and one destination | One unique sender and receiver |
| Event queue | `Queue_Size = N > 0`, FIFO | Bounded ring; one frozen item per dispatch |
| Event `Dequeue_Protocol` | `OneItem` | One `dispatch()` result |
| Event `Overflow_Handling_Protocol` | `DropNewest` or `DropOldest` | Policy type `P` |
| `Input_Time` | `Dispatch_Time` | Application aligns the port's length observation with that time |
| Thread dispatch | Periodic, sporadic, or aperiodic | Application supplies the schedule and wakeup |

One thread has one input port in this profile. The application calls
`try_send` at the source's declared output time. Other port properties or
timing modes need additional rules; [DESIGN.md](DESIGN.md) lists exclusions.

## AADL transition systems

Let `Q` be a FIFO sequence of pending events, oldest first, with
`0 <= |Q| <= N`. An event has the sole value `()`; an event data port carries
`v: T`. The following rules define both, with `v = ()` for events:

| Operation | Condition | New pending sequence | Result |
| --- | --- | --- | --- |
| `send(v)` | `|Q| < N` | `Q · v` | accepted |
| `send(v)`, `DropNewest` | `|Q| = N` | `Q` | new event dropped |
| `send(v)`, `DropOldest` | `|Q| = N` | `tail(Q) · v` | oldest event dropped |
| `dispatch` | `Q = v :: rest` | `rest` | freeze `Some(v)` |
| `dispatch` | `Q = []` | `[]` | freeze `None` |

The frozen result is stable through that dispatch. `get()` reads it without
changing it; `complete()` ends the dispatch and returns the owned event.
Dropping the guard also ends the dispatch and drops the event. Only one
dispatch can be active for an input.

A sampled data port instead has a pending value `p`, a retained visible
value `l`, and a fresh bit `u`, initially `None`, `None`, and `false`.
`send(v)` sets `p = Some(v)` and `u = true`. A dispatch freezes `p` if `u`,
otherwise `l`; it then retains that result as `l` and resets `u = false`.
The frozen value and `Updated = u` remain stable until the next dispatch.
The first dispatch returns `None` if no value was sent.

## Rust realization

| Port | Construction and dispatch | State relation |
| --- | --- | --- |
| Event | [`EventInput<P>`](src/event_input.rs) wraps `EventDataInput<(), P>` | A unit value represents one occurrence; counts and overflow still matter. |
| Event data, `DropNewest` | [`EventDataInput<T, DropNewest>`](src/event_data_input.rs) allocates capacity `N`; dispatch reads length and pops once only when positive. | Physical ring equals `Q`; a full push returns the new value for the caller to drop. |
| Event data, `DropOldest` | Allocates capacity `C = N + K - 1`; dispatch reads length, drops `max(0, length - N)` oldest values, then pops once if the count was positive. | At a dispatch input time, the newest `N` physical values are the AADL pending queue, provided no ring send fails. |
| Data | [`DataInput<T>`](src/data_input.rs) uses `EventDataInput<T, DropOldest>` with `N = 1`, then retains the last frozen value. | The newest physical value is `p`; the retained Rust value is `l`. |

The Rust [`Sender::try_send`](src/channel.rs) returns `Ok(())` for an
accepted push or `Err(v)` for a full or abandoned receiver. This mapping
assumes both endpoints stay alive. The caller drops `Err(v)` for
`DropNewest`. For `DropOldest` and data, the send bound below makes a full
ring unreachable; `Err(v)` then signals a violated assumption. The port
types do not schedule a source, wake a sporadic destination, or stage an
output.

### Send bound

For `DropOldest`, `K >= 1` bounds sends from construction to the return of
the first dispatch, and from the start of each dispatch to the return of
the next. After a dispatch, at most `N - 1` old values remain. At most `K`
new values arrive before the next dispatch returns, so occupancy is at most
`N - 1 + K = C`; no send encounters a full ring. `DataInput` uses `N = 1`,
so `C = K`.

`K` must come from a bound on arrivals, not just a thread period. If a
source releases exactly every `P`, sends at most `R` values within `J`
after each release, and every window above lasts at most `W`, a sufficient
bound is `K = R * (floor((W + J) / P) + 1)`. Include release jitter when
releases are not exact. A sporadic minimum separation alone supplies no
upper bound on `W`; its wakeup and completion latency must also be bounded.

For every port, the input time is the receiver's atomic length read inside
`dispatch()`. A zero count freezes `None` without a receive; a send published
after that read remains pending for a later dispatch. A positive count freezes
one of the values already counted, after any `DropOldest` trimming. The
application must align this read with AADL `Dispatch_Time`; entering the
function at the dispatch tick alone does not establish that timing if a send
can occur before the read.

## mCRL2 relation

| Layer | m4 model | Checked fixture |
| --- | --- | --- |
| Bounded FIFO | [channel.m4](models/channel.m4) defines `Channel(Q)` with send, receive, and length actions | [channel/fifo/app.m4](tests/channel/fifo/app.m4) |
| Event input | [event_input.m4](models/event_input.m4) instantiates a channel for unit-valued occurrences and freezes one occurrence | [event/](tests/event/) |
| Event data input | [event_data_input.m4](models/event_data_input.m4) instantiates a channel, applies queue policy, and freezes one value | [event_data/](tests/event_data/) |
| Data input | [data_input.m4](models/data_input.m4) instantiates a channel and models latest-value selection and retention | [data/sampled/app.m4](tests/data/sampled/app.m4) |

Composition has three layers. Expand `channel.m4`, one chosen port model, and
the application's `app.m4`, in that order. The app invokes one port macro;
that macro instantiates `RTCHAN_CHANNEL` and defines the port process. The
port's `*_INIT` macro joins its channel and port processes, and its `*_COMM`
macro synchronizes each caller operation with the channel transition. The app
adds its source and scheduler. The
[DropNewest fixture](tests/event_data/drop_newest/app.m4) is the smallest
complete example. For `DropNewest`, `C = N`; for `DropOldest`,
`C = N + K - 1`; for sampled data, `C = K`.

Each `send_ok(v)` or `send_full(v)` action synchronizes a source with the
channel. A positive length observation leads to `recv_ok(v)` actions between
the channel and input adapter; a zero observation leads to no receive. The
`input_begin` and `input_end` actions
mark the span of a Rust `dispatch()` call. The completion action corresponds
to `FrozenInput::complete()` or guard drop. Data adds `frozen(value,
updated)`. The mCRL2 [`comm` and `allow` operators](https://mcrl2.org/web/user_manual/language_reference/process.html#communication-and-allow)
require matching participants and block lone queue or caller actions.

The refinement target is the expanded mCRL2 `Channel || Port || App` model.
Its channel sequence `Q` represents published values not yet received, oldest
first. The following **shared FIFO contract** is the trusted `rtrb` boundary:

| `rtrb` operation | Abstract transition | Candidate observation or publication point |
| --- | --- | --- |
| Successful `push(v)` | `|Q| < C`; append `v` | Release store to tail |
| `push(v)` returning `Full(v)` | `|Q| = C`; no change | Acquire load of head |
| Successful `pop()` | Return `head(Q)` and remove it | Release store to head |
| `pop()` returning `Empty` | `Q = []`; no change | Acquire load of tail |
| Consumer `slots()` | Return `|Q|`; no change | Acquire load of tail |

Here `C` is the physical ring capacity. Each effect takes one position in an
order consistent with each endpoint's program order. An overlapping send and
dispatch can finish in the opposite order to their queue effects. Therefore
`send_ok`, `recv_ok`, and `len` label the effects above, **not method
returns**. `input_begin` labels entry into `dispatch()`, `input_end` labels
the completed freeze, and completion labels guard completion or drop.
The application must provide synchronization for any cross-thread order its
AADL timing claim requires. The trusted contract is motivated by the pinned
[`rtrb` 0.4.0 source](https://docs.rs/rtrb/0.4.0/src/rtrb/lib.rs.html):
`push` publishes a written slot through the tail, while `pop` and consumer
`slots` observe that tail before reading or counting slots.

The simulation starts with `Q = []`. At each trusted queue effect, relate the
physical FIFO to the `Q` parameter of [channel.m4](models/channel.m4). A push
appends, a pop removes the head, and a length read leaves `Q` unchanged, so
each effect matches one channel transition.

For a dispatch whose length read returns `n`, let `R` be the FIFO at that read.
Only this receiver can remove items; concurrent sends can only append after
`R`. The adapter and port model take the following same steps:

| Observation | Rust and port-model steps | Frozen result |
| --- | --- | --- |
| `n = 0` | No pop; finish the freeze | `None`; later sends stay queued |
| `n > 0`, `DropNewest` | Pop once | `R[0]` |
| `n > 0`, `DropOldest` | Pop `d = max(0, n - N)` old items, then pop once | `R[d]` |

The first `n` items stay in order until those pops, regardless of later sends.
Creusot states the selection directly as `head(R[d..n])`, with `d = 0` for
`DropNewest` and `d = max(0, n - N)` for `DropOldest`.
More precisely, after `k` trim pops (`0 <= k <= d`), the channel state is
`R[k..n] · A`, where `A` is the sequence of successful sends since the length
read. A send extends `A`; a trim pop increments `k`. After the freezing pop,
the state is `R[d+1..n] · A`. For `n = 0`, there is no pop and the state is
`A`. Creusot proves the corresponding endpoint-history equation at freeze
completion: `remaining = R[end..n] · A`, where `end` is the number of consumed
items and `A` is the suffix observed after the length read. It also proves
that the observed `R` remains a prefix during trimming and that the frozen
value is `R[d]` when `n > 0`. The link from this receiver history to the
producer's history remains a trusted `rtrb` premise. `EventInput` uses unit values.
`DataInput` uses `N = 1` and retains the selected value, matching its model's
`latest` parameter. The guard holds the frozen result through completion.
The send bound rules out physical `Full` for `DropOldest` and data.

For each admitted application schedule, the resulting obligation is
`linearized_visible(Rust) ⊆ traces(hide_internal(Channel || Port || App))`.
The independent AADL reference processes in some [finite fixtures](tests/README.md)
are model sanity checks, not another layer of this Rust-to-mCRL2 relation.

## Verification boundary

The [Creusot harness](tests/creusot/README.md) checks production Rust method
bodies against their stated contracts. The [trusted `rtrb`
contract](tests/creusot/src/rtrb_contract.rs) supplies bounded endpoint-local
FIFO histories and the values observed by receive and length operations.
Creusot proves channel outcome conversion, constructor capacities, and the
event adapter's exact selection from the FIFO sequence observed at input time.
It proves that an empty observation consumes nothing and that a positive one
consumes through the selected item, even if more sends follow the observation.
It also checks guard and data-retention contracts. These are local functional
proofs, not the complete Rust-to-mCRL2 refinement theorem.

Creusot proves both endpoint histories start empty, but its current contracts
do not establish the shared FIFO contract above: the producer's accepted
history is not linked to the consumer's offered history. This contract is
trusted because `rtrb` is outside the proof. A previously trusted pair axiom
was removed after a consistency test showed it could prove `false`. The
simulation argument above is conditional on the stated boundary, send bound,
and application timing; it is not a machine-checked trace-refinement theorem.
See the [proof boundary](tests/creusot/README.md) for the exact status.

Endpoint abandonment, `mem::forget` of a frozen guard, send-bound violations,
and user-defined `T` destructors are outside the mCRL2 relation. Model
deadlock freedom also does not establish Rust scheduling progress.
