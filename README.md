# io-uring-x1

Experiments with Linux io_uring from its primitives, in Rust. Early days: the workspace builds
and validates, and the io_uring work is in `TODO.md`.

## Tasks

`cargo xtask <task>` runs the project's own tasks, the `xtask` package, aliased in
`.cargo/config.toml`:

- `cargo xtask version` prints the version in `Cargo.toml`.
- `cargo xtask bump <next|bare|VERSION>` sets it and refreshes `Cargo.lock`.
- `cargo xtask completions [NAME...]` prints a bash completion function for `cargo xtask`.

`vc-x1 validate`, `vc-x1 build`, and `vc-x1 install` read `.vc-config.md`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
