# io-uring-x1

Experiments with Linux io_uring from its primitives, in Rust, toward io_uring as the kernel
interface of a message-based actor system built on [zc-ring-x1](https://github.com/winksaville/zc-ring-x1)'s pool and queues.

- `src/ring.rs`: an io_uring set up, mapped, and driven by hand with rustix, at chosen queue depths.
- `src/driver.rs`: NOPs carried to the ring and back over two zc-ring-x1 spsc queues.
- `io-uring-x1 [--sq-entries N] [--cq-entries N]` runs a caller and the driver on two threads and
  prints the round trip.

How it works, and the memory rules zero-copy rests on, is [notes/io-uring.md](notes/io-uring.md).
zc-ring-x1 is a path dependency on the sibling checkout `../zc-ring-x1`.

## Tasks

`cargo xtask <task>` runs the project's own tasks, the `xtask` package over the `xtask-lib`
library, aliased in `.cargo/config.toml`:

- `cargo xtask version` prints the version in `Cargo.toml`.
- `cargo xtask bump <next|bare|VERSION>` sets it and refreshes `Cargo.lock`.
- `cargo xtask validate [--fast]` validates the workspace, read-only: the format check, clippy,
  and the tests, or with `--fast` the tests alone.
- `cargo xtask run [--bin NAME] [ARGS...]` runs the workspace's binary, its arguments and `-h`
  passed as `cargo run` passes them, `cargo xtask help run` for its own help.
- `cargo xtask completions [NAME...]` prints a bash completion function for `cargo xtask`.

`vc-x1 build` and `vc-x1 install` read `.vc-config.md`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
