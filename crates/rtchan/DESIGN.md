# Design decisions

The public AADL primitives are `EventInput<P>`, `EventDataInput<T, P>`, and
`DataInput<T>`. Each owns one receiving endpoint; its sender belongs to one
source. A private implementation relationship keeps them small:
`EventInput<P>` uses `EventDataInput<(), P>`, and `DataInput<T>` uses
`EventDataInput<T, DropOldest>` with queue size one. The shared concurrent
storage is the Rust SPSC [`channel`](src/channel.rs). The [mCRL2 channel](models/channel.m4)
defines that FIFO separately. Each [port model](models/) instantiates it and
defines its own input process. The app instantiates one port and supplies the
source and dispatch schedule.

| Choice | Reason |
| --- | --- |
| One source, one destination | Matches SPSC ownership and avoids an unspecified fan-in order. |
| `N > 0`, FIFO, `OneItem` | One dispatch freezes one oldest pending occurrence. These values give bounded, explicit queue behavior. |
| `DropNewest` or `DropOldest` | Both AADL policies are useful. `DropNewest` is one failed push; `DropOldest` trims at dispatch because an SPSC producer cannot evict the consumer-owned oldest entry. |
| A bound `K` for `DropOldest` and data | Extra capacity prevents a send from seeing a full physical ring. Without it, dispatch-time trimming cannot reproduce AADL send-time eviction or latest-value sampling. |
| Data as retained last value | Queue size one plus a retained value implements sampled data without another concurrent structure. |
| Scheduling outside `rtchan` | AADL `Input_Time`, `Output_Time`, and sporadic separation are application and scheduler obligations, not queue operations. |

Construction allocates. Afterwards, `try_send` and `try_recv` perform one
wait-free `rtrb` operation. `DropOldest` dispatch can discard at most `K - 1`
old entries; data dispatch can replace its retained value. Destructors of
`T` may execute during these calls and must be budgeted by a real-time
application.

The supported property values and transition rules are in [MAP.md](MAP.md).
The profile excludes `Queue_Size => 0`, fan-in, multiple input ports frozen
at one instant, `MultipleItems`/`AllItems`, non-FIFO queues, `Error`
overflow, mode changes, and bus transport. Fan-out needs one port per
destination and application-level sends. Timed, hybrid, and interrupt
dispatch need scheduler models before claiming a mapping.
