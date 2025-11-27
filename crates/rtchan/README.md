# rtchan

`rtchan` implements three AADL input-port kinds with one bounded SPSC queue
per connection. It is `no_std`; queue storage is allocated at construction.
The queue operations do not allocate or wait internally. A dispatch may drop
payloads, whose destructors have application-defined cost. Construct and
destroy ports outside real-time code.

| AADL input | Rust type | Value read at dispatch |
| --- | --- | --- |
| Event | `EventInput<P>` | One occurrence, represented by `()` |
| Event data | `EventDataInput<T, P>` | One queued `T` |
| Data, sampled | `DataInput<T>` | Latest `T`, retained across dispatches |

`P` is `DropNewest` or `DropOldest`. An event or event data port freezes one
item per dispatch (`Dequeue_Protocol => OneItem`). A data port reports whether
a new value arrived through `FrozenData::is_updated()`.

```rust
use rtchan::{DataInput, DropNewest, EventDataInput, EventInput};

let (mut event_sender, mut event) = EventInput::<DropNewest>::new(2);
event_sender.try_send(()).unwrap();
assert_eq!(event.dispatch().unwrap().complete(), Some(()));

let (mut message_sender, mut message) = EventDataInput::<u8, DropNewest>::new(2);
message_sender.try_send(7).unwrap();
assert_eq!(message.dispatch().unwrap().complete(), Some(7));

let (mut value_sender, mut value) = DataInput::<u8>::new(2);
value_sender.try_send(9).unwrap();
assert_eq!(value.dispatch().get(), Some(&9));
```

An event or event data input with `DropNewest` uses a ring of capacity `N`.
A full send returns its value to the caller, which drops it. With
`DropOldest`, construction takes `(N, K)`, uses capacity `N + K - 1`, and
discards excess old entries at dispatch. `DataInput::new(K)`
uses capacity `K`. For the latter two, `K` must bound sends between dispatch
windows; [MAP.md](MAP.md#send-bound) defines the window and proof obligation.
All constructors require positive sizes and panic on invalid sizes.

The bare [`channel`](src/channel.rs) remains available as the storage
primitive. Its [mCRL2 model](models/channel.m4) composes with one of the
[input adapters](models/) and the application's source and scheduler
processes; [a fixture](tests/event_data/drop_newest/app.m4) shows the
composition. `rtchan` does not schedule threads, stage outputs, or wake
sporadic consumers. Applications call `try_send` and `dispatch` at the
declared AADL times.

- [DESIGN.md](DESIGN.md) explains the chosen profile and exclusions.
- [MAP.md](MAP.md) defines the transition systems and correspondence.
- [tests/README.md](tests/README.md) lists the port fixtures and proofs.
