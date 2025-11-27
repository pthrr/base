# base

This workspace contains four Rust crates:

| Crate | Public API | Runtime requirements |
| --- | --- | --- |
| [`base-error`](crates/base-error/src/lib.rs) | `Error`, `ArithmeticError`, `LookupError` | `no_std`, no allocator |
| [`base-macros`](crates/base-macros/src/lib.rs) | `invoke!` | `no_std`, no allocator |
| [`base`](crates/base/src/lib.rs) | Reexports `base::error`, `base::macros`, `base::invoke!`, and `base::rtchan` | `no_std`; channel construction allocates |
| [`rtchan`](crates/rtchan/README.md) | AADL `EventInput`, `EventDataInput`, and `DataInput` over a bounded SPSC channel | `no_std`; allocates its buffer at construction |

Use `base::rtchan` for channels and the AADL port adapters when the
application already depends on `base`. `rtchan` also remains an independent
crate. For an application beside this checkout:

```toml
[dependencies]
base = { path = "../base/crates/base" }
```

Adjust the paths to the application's location. The [rtchan API guide](crates/rtchan/README.md)
shows a working example. Its [AADL mapping](crates/rtchan/MAP.md) defines the
supported port profile and the Rust–mCRL2 correspondence.

## Checks

With Rust, [Go Task](https://taskfile.dev/), and [Nix](https://nixos.org/) installed,
run `task install` once for the Rust target and formatter, then `task ci`.
The [Taskfile](Taskfile.yml) runs formatting and host and embedded checks;
`task test` runs the unit, mCRL2, and Creusot suites. Run one suite with
`task test:unit`, `task test:mcrl2`, or `task test:creusot`. The
[GitHub workflow](.github/workflows/ci.yml) runs the same task; pre-commit
also invokes it. The [flake](flake.nix) defines the model-checking shell for
x86_64 and aarch64 Linux and the proof shell for x86_64 Linux only, so the
full `task ci` runs on x86_64 Linux.

The [model checks](crates/rtchan/tests/README.md) establish properties of the supplied
fixtures, and the [Creusot proof](crates/rtchan/tests/creusot/README.md)
checks the channel and adapter bodies against stated contracts, with `rtrb`
trusted. The [rtchan mapping](crates/rtchan/MAP.md#verification-boundary)
states what is verified overall and what remains trusted.
