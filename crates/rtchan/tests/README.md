# Port verification

`task test:mcrl2` expands the m4 channel and port models and checks their
mCRL2 properties. `task test:creusot` runs [Creusot](creusot/README.md) on
production Rust code. `task test:unit` runs the Rust tests. `task test` runs
all three; `task ci` also runs formatting and embedded checks.

Each `app.m4` is a closed test application: it composes reusable models with
source and dispatch processes. The reusable definitions live in [models/](../models/).

| Model | Fixture | Main checks |
| --- | --- | --- |
| Bounded channel | [channel/fifo/app.m4](channel/fifo/app.m4) | FIFO order, full and length outcomes, no deadlock. |
| Event, `DropNewest` | [event/drop_newest/app.m4](event/drop_newest/app.m4) | Singleton occurrence freezes correctly; empty completion; no deadlock. |
| Event, `DropOldest` | [event/drop_oldest/app.m4](event/drop_oldest/app.m4) | No physical overflow and weak-trace equivalence with a two-entry AADL event queue. |
| Event data, `DropNewest` | [event_data/drop_newest/app.m4](event_data/drop_newest/app.m4) | A valued item stays frozen, completion matches it, no deadlock. |
| Event data, `DropOldest` | [event_data/drop_oldest/app.m4](event_data/drop_oldest/app.m4) | No physical overflow and weak-trace equivalence with an AADL queue for `N = K = 2`. |
| Event data, sporadic | [event_data/sporadic/app.m4](event_data/sporadic/app.m4) | Remembered wakeup drains accepted sends without empty dispatch. |
| Event data, sporadic `DropOldest` | [event_data/sporadic_drop_oldest/app.m4](event_data/sporadic_drop_oldest/app.m4) | Trimming, no overflow, newest-event completion, and weak-trace equivalence. |
| Data, sampled | [data/sampled/app.m4](data/sampled/app.m4) | No overflow and weak-trace equivalence with a latest-value register for `K = 2`. |

Each fixture has a `properties/` directory; `task test:mcrl2` checks every `.mcf`
file in it. The finite
fixtures use Boolean or small enumerated payloads. Their source processes
enforce the stated send bounds where needed. Weak-trace comparison hides
internal length, receive, and retention steps; it does not establish
fairness or timing. The [mapping](../MAP.md#mcrl2-relation) states the
conditions for applying these results to Rust.
