# Todo and cycle record

This file contains near term tasks with a short description and reference links to more details.
Its shape is [Todo format](agent-data/notes.md#todo-format).

## Continuation notes

Where the agent was, for the agent that comes next. Ephemeral, never a record, read first at
acquaint, acted on, and reset to `_None._` by the reader.

_None._

## In Progress

A cycle's record has one home at a time, and while the cycle runs this is it. The block's shape is
the specimen in [cycle-model.md](agent-data/cycle-model.md), and the rules are in [The In Progress
block](agent-data/notes.md#the-in-progress-block).

### feat: issue IORING_OP_NOP via io_uring primitives

#### Problem

The long term goal is to use io_uring as the interface to the kernel for our message
based actor system with our latest versions of pool, mpsc and spsc in ../zc-ring-x1

The short term goal is to start with IORING_OP_NOP ignoring our pool ... and then
try to connect two of our spsc to and to send IORING_OP_NOP via the sqe (Submission Queue Entry)
and receive it via cqe (Completion Queue Entry)

Use rustix:
- repo: https://github.com/bytecodealliance/rustix
- docs: https://docs.rs/rustix/latest/rustix/

More information is at Linux io_uring docs:
- io_uring: https://man7.org/linux/man-pages/man7/io_uring.7.html
- io_uring_enter and the opcodes: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html

#### Solution

Build the io_uring from its primitives with rustix: `io_uring_setup`, the SQ and CQ rings and the
SQE array mapped by `mmap`, and `io_uring_enter` to submit and wait. Issue `IORING_OP_NOP` and reap
its CQE, then put two zc-ring-x1 spsc queues in front of the ring, one carrying NOP requests in to
the SQ and one carrying the completions out from the CQ.

#### Acceptance check

`cargo test` passes, including a test that pushes NOP requests, each with its own `user_data`, into
the request spsc and reads back from the completion spsc one CQE per request, each with the matching
`user_data` and `res` 0. Running `io-uring-x1` prints the same round trip.

#### Deliberation

- Primitives, not a wrapper crate: rustix's `io_uring` module supplies the syscalls and the kernel's
  structs, and the ring setup and index handling are ours.
  - The `io-uring` crate and liburing do the mapping and the head and tail protocol for us, and
    learning that protocol is the point, since the actor system will drive it directly.
- Three work rungs, each with its own test: the ring set up, a NOP round trip, the spsc queues in
  front of it.
  - The spsc rung needs a working ring under it to test against, so the ring comes first.
- One mapping for both rings: we require `IORING_FEAT_SINGLE_MMAP`, which every kernel since 5.4
  reports, and refuse a kernel without it rather than carry the two-mapping path.
- The spsc is zc-ring-x1's by path dependency, `../zc-ring-x1`, its default version, revisited in
  that rung.
- A rung inserted after the ring's: validation moves into `cargo xtask validate`, read-only.
  - It was asked for mid-cycle, vc-x1 keeping out of the repo's content, and the old fast run
    skipped the new library's tests, so it lands before more ring code needs validating.
- Version 0.2.0: a feature over 0.1.0, so the opening is `0.2.0-0` and the rungs count up from it.

#### Rungs

The rung subsections, in the ladder's order. The ladder itself is in `zTOC.md > ## Ladder`.

##### feat: issue IORING_OP_NOP via io_uring primitives opening

The cycle's first commit, carrying its plan: the bookmark, the last cycle's record deleted, and the
Todo entry moved into this block.

##### feat: set up an io_uring with rustix

A ring that can be set up and torn down: rustix as a dependency, `io_uring_setup`, the rings and the
SQE array mapped, and the kernel's offsets resolved into the head, tail, mask, and array pointers,
unmapped on drop.

* The ring lives in a library, `src/ring.rs` under `src/lib.rs`, with `main` a client of it.
  - The later rungs and their tests reach the ring as `io_uring_x1::ring`, and a public item is
    never dead code while its users are still to come.
* `Ring::new` sets up, checks `IORING_FEAT_SINGLE_MMAP`, and maps twice.
  - The rings' mapping is sized for the larger of the SQ, ending at its index array, and the CQ,
    ending at its CQE array, and the SQE array is its own mapping at `IORING_OFF_SQES`.
  - The heads and tails are held as `*const AtomicU32` into the mapping, since the kernel writes
    them concurrently, and the masks and entry counts are read once, since they never change.
  - The SQ index array is filled once with slot i naming SQE i, so the next rung publishes an SQE
    by its slot and never writes the array again.
* `Ring::state` reads the indexes and sizes, the test's view and `main`'s output.
  - The tests check the kernel's sizes, 8 SQEs and 16 CQEs, and that 5 entries round up to 8.
* `Drop` unmaps both mappings before the fd closes, a failed `munmap` ignored as a leak at worst.

##### feat: validate by cargo xtask

Validation is the project's knowledge, held natively by its own tool, not by vc-x1's config:
`cargo xtask validate` replaces `[validate]` in `.vc-config.md`, read-only, with `cargo fmt` moved
into the Work step, and its `--fast` runs the library's tests the old `cargo test --bins` skipped.

* vc-x1 is moving to know nothing of a repo's content but its version, each project's own tooling
  doing the rest: cargo here, cmake for a C program, an xtask where the build is sophisticated.
  - vc-x1 reads `[validate]` only for `vc-x1 validate`, no cycle stage gating on it, so removing
    the table removes that command's use here and nothing else.
* `cargo xtask validate` runs `cargo fmt --all --check`, clippy over the workspace's targets with
  warnings denied, and `cargo test --workspace`, each announced and the first failure stopping it.
  - Read-only: validation runs before a review, before a push, and on the closed tree, and one that
    rewrites can change a tree after its review. `cargo fmt` is the Work step's last act instead.
  - `--fast` is `cargo test --workspace --lib --bins`, the ring's tests now included.
  - A test pins the commands, the format checked and never applied, and the completion test now
    expects `v` to offer `version` and `validate`, `ve` the one.
* `custom.md` carries the override, since `AGENTS.md` names `vc-x1 validate` and `[validate]` in the
  per-rung flow, Local ladders, and Before any push, and a change there is an agent-files proposal.

##### fix: restore clap's completion script layout

The completion script's `COMPREPLY` line lost clap's `\` continuations, leaving runs of spaces, and
its `IFS` holds a raw vertical tab where clap writes `$'\013'`, both an escape-processing accident
in the copy from vc-x1. Restore clap's text in the raw string, and test for the escape as written.

* The damage is what a non-raw string makes of clap's text: `\` and its newline dropped with the
  next line's indent kept, and `\013` turned into the byte it names.
  - It ran the same, bash ignoring the spaces and reading the byte inside `$''` as itself, and the
    risk was the invisible byte, which an editor or formatter could strip and silently change the
    word splitting of every completion.
* The script is clap's again: `local IFS=$'\013'` as text, and the completer's environment one
  assignment per line, joined by `\`, in the raw string where a backslash stays itself.
* A test pins it: the escape as its four characters, no raw vertical tab, the continuations in
  place, and no run of spaces mid-line. The printed script passes `bash -n` and completes `cx v`
  to `version` and `validate`.
* vc-x1's own xtask carries the same copy, `xtask/src/main.rs`, told to it only if Wink says so.

##### feat: submit IORING_OP_NOP and reap its CQE

A NOP round trip: an SQE written and published by the SQ tail, `io_uring_enter` to submit and wait,
the CQE read and the CQ head advanced, each NOP's `user_data` coming back with `res` 0.

* Each ring has one producer and one consumer, each owning one index: we own the SQ tail and the CQ
  head, the kernel the SQ head and the CQ tail.
  - Our own index is read `Relaxed`, since our last store is current, and the kernel's `Acquire`,
    so the entry it covers is seen. Ours is stored `Release` after the entry is written or read,
    so the kernel sees a whole SQE, and never reuses a CQE slot before we have copied it out.
  - The indexes run free and wrap, a slot being the index masked, so a full SQ is `tail - head`
    equal to the entry count, and the test runs three full rounds of a 4-entry ring to cross it.
* `push` publishes an SQE, refusing with `SqFull` when no slot is free, `submit(want)` hands the
  published SQEs to `io_uring_enter` and waits for `want` completions, and `pop` copies the oldest
  CQE out as a `Completion` before advancing the head.
  - `unsubmitted` counts what is published and not yet entered, since the kernel may consume fewer
    than it is offered, and `submit` returns what it took.
  - `nop(user_data)` builds the SQE, every field zeroed but the opcode and the `user_data`.
* The index loads go through one `unsafe fn load`, each caller vouching for its pointer, since a
  safe function dereferencing any pointer it is handed would be unsound.
* `main` sends four NOPs through an 8-entry ring and prints each completion and the indexes, all four
  meeting at 4.

##### feat: run the one binary by cargo xtask

`cargo run` in this workspace sees two binaries, the project's and xtask's, and asks for `--bin`.
`cargo xtask run [-- ARGS]` finds the workspace's binaries by `cargo metadata`, xtask's left out,
and runs the one there is, refusing with the list when there is not exactly one.

* The binaries are read from `cargo metadata --no-deps`, whose packages are the workspace's members,
  each one's targets of kind `bin`, the package named as xtask's own (`CARGO_PKG_NAME`) left out,
  so the libraries and the task runner never count.
  - `serde_json` reads the JSON, xtask's one new dependency.
  - Exactly one binary runs by `cargo run --bin <it> -- ARGS`, with cargo's own build output, so it
    reads as `cargo run` does. None or several are refused, the list given, and `--bin NAME` picks
    one, cargo then answering a wrong name with its own list.
* ARGS come after `--` (clap's `last`), so a flag of the binary's is never taken for xtask's.
* `run` returns nothing to print, and `main` now prints a task's text only when there is some, so
  the binary's output is the run's last line.
* A test feeds `binaries` a metadata sample: the one binary found with the lib and xtask left out,
  two listed when nothing is skipped, none from empty JSON.

##### refactor: make xtask a library

The tasks live in `xtask/src/main.rs`, so another project cannot reuse them. Move them into a
package of their own, `xtask-lib`, as modules, xtask's `main` keeping the arguments and the
dispatch, the workspace root found at runtime rather than compiled in, and what is this project's
own passed in, not built in.

* `xtask-lib` is its own workspace member, and `xtask` depends on it by path, so the boundary is
  real: the binary reaches only what the library makes public.
  - The library holds `toml_edit` and `serde_json`, and the binary only clap and the library.
  - It has its own version, `0.1.0`, and `cargo xtask bump` sets the root package's alone, the
    version-of-record.
  - Named `xtask-lib`, not `xtask-x1`: `-x1` marks an experiment, dropped at 1.0, so the lasting
    name is taken now, `-lib` telling it from each repo's `xtask` binary package.
  - Moving it into its own repo, a path dependency becoming a git one, is the Todo `feat: share the
    xtask library across repos`.
* `xtask-lib/src/lib.rs` has a module per task, each with its tests beside it: `version` (the
  version and its bump), `validate`, `run`, and `completions`, plus `cargo()` and `workspace_root()`.
  - `xtask/src/main.rs` keeps clap's `Cli` and `Task`, the bump word candidates, the dispatch, and
    the tests of argument parsing and completion, which need the `Cli`.
  - Another project's xtask depends on the library and writes its own `main`, choosing its tasks.
* The root is found at runtime by `cargo locate-project --workspace`, not compiled in from
  `CARGO_MANIFEST_DIR`, which in a dependency would name the library's directory, not the workspace
  using it. `cargo xtask version` from `src/` and `run` from `xtask/` both find this workspace.
* What is a project's own is passed in: `validate` takes its commands and a label, `commands(fast)`
  the default set, and `run` takes the package to leave out, `main` passing its own
  `CARGO_PKG_NAME`.
* The tests that read the real workspace stay generic, so they hold wherever the library goes: the
  root holds a `[workspace]` manifest, and leaving out a package with binaries drops exactly its
  binaries, whichever package that is.
* `completions` needs no root, so a tab press runs no `cargo locate-project`.

##### fix: pass run's arguments as cargo run does

`cargo xtask run` takes the binary's arguments only after `--`, where `cargo run` passes the first
plain argument and everything after it. Make `run` take them the same way, `--` needed only when
the binary's first argument is a flag or one of `run`'s own.

* `args` is clap's `trailing_var_arg` with `allow_hyphen_values`, in place of `last`: the first
  argument that is not `run`'s own starts the binary's, and everything after it is the binary's,
  flags and a later `--bin` included, as cargo does.
  - `--` is needed only before one of `run`'s own, `--bin`, `-h`, or `--help`: `cx run -- --help`
    hands `--help` to the binary, `cx run --help` shows `run`'s.
  - The cost: an unknown flag first, a mistyped `--bni` say, reaches the binary rather than being
    refused, an acceptable trade for a run wrapper.
* The parse test pins each case, and cargo's own `Running` line shows the binary given `foo -x --bin
  b` and `--help`.
* Completion follows the same split.
  - `cx run --bin <tab>` offers the workspace's binaries, read by `cargo metadata` on each tab,
    xtask's own left out, by `xtask-lib`'s `metadata`, now public.
  - Once the binary's arguments have begun, `cx run foo <tab>` and `cx run -- <tab>` offer nothing,
    where they offered `--bin` and `--help`, which would have reached the binary.
  - clap's engine ignores `trailing_var_arg` when completing, offering a command's options beside
    any positional whose minimum is met, always for a `Vec`, and has no knob for it. So `main`
    answers such a request with nothing before clap is asked, `xtask-lib`'s
    `binary_args_started` deciding it from the request's words and index.
  - Completing the binary's own arguments waits for it to have a CLI, `io-uring-x1` having none.
* The earlier rung, `feat: run the one binary by cargo xtask`, said ARGS come after `--`; that
  subsection stays as it was, this rung superseding it.

##### feat: choose the ring's queue depths

The ring's depths are fixed, 8 SQEs and the kernel's default of twice as many CQEs. Make both
optional: an SQ depth, and a CQ depth by `IORING_SETUP_CQSIZE`, on `Ring` and as the binary's
`--sq-entries` and `--cq-entries`, its first CLI, so `cx run -- ...` has something to pass.

* `Depths { sq, cq: Option<u32> }` names the depths, its default 8 and the kernel's twice-the-SQ,
  and `Ring::with_depths` sets them up, `Ring::new(entries)` staying as the SQ-only shorthand.
  - A named CQ is asked for by `IORING_SETUP_CQSIZE` with `cq_entries`, the kernel rounding it up
    to a power of two as it does the SQ's.
  - Tests pin the kernel's answers: a CQ of 32 over an SQ of 4, 5 rounded to 8, a CQ equal to
    the SQ, and the refusals, a CQ smaller than the SQ and a zero depth either way, each
    `EINVAL`.
* The binary has a clap CLI, `--sq-entries N` (default 8) and `--cq-entries N`, and a test of its
  parsing.
  - Its `-V` is clap's `version`, since `vc-x1 install` runs `-V` to read the version it installs,
    and a CLI without it refused the flag and failed the install.
  - The round trip sends its four NOPs in batches the SQ holds, submitting and reaping each, so an
    SQ smaller than four works, `--sq-entries 2` taking two batches.
  - The kernel's refusal is a bare `EINVAL`, so `main` names the depths it refused and the rule
    beside it.
* `cx run -- --sq-entries 2 --cq-entries 16` passes them through, and completion offers nothing
  after the binary's first flag, the binary still having no completion of its own.

##### fix: hand run's -h to the binary

`cx run -h` shows `run`'s help, where the app's is wanted, reached today only by `cx run -- -h`.
Turn off `run`'s own help flag so `-h` and `--help` go to the binary, `run`'s help moving to `cx help
run`, and completion treating them as the binary's.

* `run` has clap's `disable_help_flag`, so `-h` and `--help` are ordinary arguments and, with
  `allow_hyphen_values`, the binary's: `cx run -h` and `cx run --bin io-uring-x1 -h` print the app's
  help, as `cx run -- -h` did.
  - `run`'s own help is clap's `help` task, `cx help run`, and its long help says so.
  - `--bin` is `run`'s one flag, and `--` is needed only before a `--bin` meant for the binary.
  - A difference from cargo, accepted: `cargo run -h` is cargo's help, and `cx run -h` the app's,
    since `run` is a wrapper whose only option is `--bin`.
* `binary_args_started` no longer counts `-h` and `--help` as `run`'s, so `cx run <tab>` offers
  `--bin` alone and `cx run -h <tab>` nothing.
* The parse test pins `-h` and `--help` reaching the binary, with and without `--bin`, and `help
  run` showing `--bin <NAME>`.

##### feat: carry NOPs over zc-ring-x1 spsc queues

Two spsc queues in front of the ring: a request queue whose entries become SQEs, and a completion
queue the CQEs are copied into, so a caller never touches the ring itself.

* `src/driver.rs`'s `Driver` owns the io_uring, the request queue's consumer, and the completion
  queue's producer, and `step(wait)` moves what it can: requests into SQEs, the SQ into the kernel,
  CQEs into completions. `NopRequest { user_data }` and `NopCompletion { user_data, res, flags }`
  are the slots' `#[repr(C)]` zerocopy messages.
  - Backpressure, nothing dropped: a request is released only once its SQE is published, a read
    slot dropped otherwise re-delivering it, and a completion slot is reserved before its CQE is
    popped, a write slot dropped otherwise abandoning it, so a full completion queue leaves CQEs
    in the CQ.
  - `in_flight` is kept at most the CQ's depth, so the kernel never holds more completions than
    the CQ has room for, no CQ overflow possible.
  - `step(true)` with something in flight waits in the kernel for one completion, so an idle
    driver sleeps rather than spins.
* The spsc is zc-ring-x1's default, v3, built from a `Pool` over a line-aligned `Vec`, `store`
  sizing it, one segment per queue, in-process.
  - zc-ring-x1 is `no_std`, its errors not `std::error::Error`, so the tests turn errors into their
    `Debug` text and `main` names each queue's.
  - It is a path dependency on the sibling checkout `../zc-ring-x1`, so a build takes whatever that
    working copy holds, here its `main` at `e705b50d`, version `0.19.5`, and a clone without the
    sibling does not build. Pinning it, a git dependency at a commit, is for when zc-ring-x1's API settles.
* Two tests: NOPs cross both queues on one thread, six through a 4-entry ring, and a completion
  queue of 2 holds CQEs back, 4 in flight and 2 completed, until the caller drains all 8. The
  second assumes a NOP completes during its submit, which 200 runs bore out.
* `main` runs the caller and the driver on two threads, the io_uring made on the driver's, its
  pointers never crossing, the spsc endpoints `Send`. The caller's waits end if the driver stops,
  so a refused setup reports and exits rather than hanging, and the output's order is fixed,
  setup, completions, final state. It runs at one-entry rings, 100 runs.

##### feat: issue IORING_OP_NOP via io_uring primitives closing

Closing out the cycle.

## Waiting

Important work that cannot start yet. Each entry names what it waits on, in a form that can be
checked, and the rank it takes in `## Todo` once unblocked. Every opening checks each condition and
promotes what is met ([Opening](AGENTS.md#opening)).

## Todo

Entries are in priority order, the first highest, and reprioritizing is moving an entry. Each is a
`###` heading, so a citation is a link to its anchor. Long-tail entries live in
[todo-backlog.md](notes/todo-backlog.md). Use the [Prose form](agent-data/prose.md#prose-form).

### feat: build and install by cargo xtask

`[build]`, `[install]`, and `[profile.*]` in `.vc-config.md` are the project's knowledge in vc-x1's
config, as `[validate]` was before `cargo xtask validate`. Move them into xtask, `cargo xtask install
--profile dev` for one, with a `custom.md` override where the agent-files name `vc-x1 install`.

### fix: a cycle commit marks no rung current

`vc-x1 cycle --commit` writes each rung `(done)` and the next `(current)` into the commit's
`zTOC.md`, so commit N records a claim about rung N+1, work not yet begun. It goes stale when the
plan changes, as it did in the `feat: issue IORING_OP_NOP via io_uring primitives` cycle, where the
ring rung's commit named the NOP rung current and two rungs were then inserted ahead of it. The
marker is also redundant, `cycle --status` computing the next rung from the branch, and a rung's own
diff never touches the ladder. Proposed: a commit marks its own rung done and nothing current.
It is vc-x1's code and the agent-files' wording (`AGENTS.md` per-rung flow step 7 and Cycle-record,
`agent-data/notes.md` zTOC format), so it goes to vc-x1 as a message and an agent-files proposal
when Wink says so.

### build: pin zc-ring-x1 by git, not the sibling path

`zc-ring-x1` is a path dependency on `../zc-ring-x1`, so a build takes whatever that working copy
holds and a clone without the sibling does not build. Its `main@origin` is `e705b50d`, the commit
the spsc rung was built on, so `git = "https://github.com/winksaville/zc-ring-x1.git"` with that
`rev` works now, and a `tag` once zc-ring-x1 tags its versions. zc-ring-x1's side, tags and
`core::error::Error` for its error types (`no_std` allows it since Rust 1.81, and `?` into `Box<dyn
Error>` needs it), is its own to record, asked of it by message when Wink says so.

### refactor: read cargo metadata into typed structs

`xtask-lib`'s `run` reads `cargo metadata` as a `serde_json::Value`, indexed by name, so a typo or a
schema change reads as a silent `Null`, and `binaries` falls back with `unwrap_or(&empty)` twice.
Deserialize into `Metadata`, `Package`, and `Target` structs with serde's derive, owned `String`s,
so a missing field is a parse error and the fallbacks go. Clarity, not speed: the parse is
microseconds beside the `cargo metadata` process.

### feat: share the xtask library across repos

This cycle makes xtask a library within this workspace. Moving it into its own repo, a git or path
dependency, lets vc-x1, whose xtask this one was modelled on, and other projects use one copy.

## Ideas

Items not yet solid enough for `## Todo`. Triaged at the next opening: promote, fold into a
picked-up cycle, or drop.

## Bugs

_See [bugs.md](notes/bugs.md).

## Closed

The last cycle's finished record, moved here whole by its closing commit and deleted by the next
opening ([Cycle-record](AGENTS.md#cycle-record)).

# References
