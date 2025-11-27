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
| `Input_Time` | `Dispatch_Time` | Application calls `dispatch()` at that time |
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
| Event data, `DropNewest` | [`EventDataInput<T, DropNewest>`](src/event_data_input.rs) allocates capacity `N`; dispatch pops once. | Physical ring equals `Q`; a full push returns the new value for the caller to drop. |
| Event data, `DropOldest` | Allocates capacity `C = N + K - 1`; dispatch reads length, drops `max(0, length - N)` oldest values, then pops one. | At a dispatch input time, the newest `N` physical values are the AADL pending queue, provided no ring send fails. |
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

The `DropOldest` input time is the length read when values are queued. A
send during the subsequent trimming remains pending for a later dispatch.
When the ring was empty at that read, the freezing receive is the input
time. The application must place the call so this linearization point is
the AADL input time. `DropNewest` needs only the freezing receive.

## mCRL2 relation

| Layer | m4 model | Checked fixture |
| --- | --- | --- |
| Bounded FIFO | [channel.m4](models/channel.m4) defines `Channel(Q)` with send, receive, and length actions | [channel/fifo/app.m4](tests/channel/fifo/app.m4) |
| Event input | [event_input.m4](models/event_input.m4) gives occurrences a singleton sort and reuses the event data adapter | [event/](tests/event/) |
| Event data input | [event_data_input.m4](models/event_data_input.m4) reads and trims the channel, then freezes one value | [event_data/](tests/event_data/) |
| Data input | [data_input.m4](models/data_input.m4) retains the last frozen value with `N = 1` | [data/sampled/app.m4](tests/data/sampled/app.m4) |

Composition is explicit: expand `channel.m4` before the chosen input model;
instantiate `RTCHAN_CHANNEL(port, T, C)` and the input macro with the same
prefix and payload sort; then compose their `*_INIT` processes with the
application's source and scheduler. `RTCHAN_CHANNEL_COMM(port)` merges each
caller operation with the matching channel transition. The
[DropNewest fixture](tests/event_data/drop_newest/app.m4) is the smallest
complete example. For `DropNewest`, `C = N`; for `DropOldest`,
`C = N + K - 1`; for sampled data, `C = K`.

Each `send_ok(v)` or `send_full(v)` action synchronizes a source with the
channel; each `recv_ok(v)` or `recv_empty` action synchronizes the channel
with the input adapter. The `input_begin` and `input_end` actions
mark the span of a Rust `dispatch()` call. The completion action corresponds
to `FrozenInput::complete()` or guard drop. Data adds `frozen(value,
updated)`. The mCRL2 [`comm` and `allow` operators](https://mcrl2.org/web/user_manual/language_reference/process.html#communication-and-allow)
require matching participants and block lone queue or caller actions.

For a live SPSC pair, relate the ordered contents of `rtrb` to the model's
`Channel` sequence, the Rust guard to the model state between
`input_end` and completion, and `DataInput`'s retained value to the model's
`latest` parameter. Erasing `len` and receive bookkeeping gives the AADL
observations. The [fixtures](tests/README.md) check the finite channel and
port models. For `DropOldest` event and event data ports and sampled data,
they also check weak-trace equivalence with direct AADL reference processes
under an explicit send bound. These checks
do not prove the relation for every payload or schedule.

## Verification boundary

The [Creusot harness](tests/creusot/README.md) checks production Rust method
bodies against their stated contracts. Its [trusted `rtrb` contract](tests/creusot/src/rtrb_contract.rs)
states accepted and delivered histories, FIFO delivery, capacity bounds, and
their relation when both endpoints are quiescent. Creusot proves that the
channel wrapper preserves those assumed histories, along with constructor
bounds, result conversion, trim arithmetic, and latest-value updates in the
port adapters. It does not verify `rtrb`'s implementation or prove complete
AADL port refinement. Unique endpoint ownership and correct application
timing are further assumptions. The induction from these assumptions and the
Rust steps to the port model is an argument, not a machine-checked
cross-language proof.

Endpoint abandonment, `mem::forget` of a frozen guard, send-bound violations,
and user-defined `T` destructors are outside the mCRL2 relation. Model
deadlock freedom also does not establish Rust scheduling progress.
