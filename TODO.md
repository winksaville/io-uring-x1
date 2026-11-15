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

##### feat: carry NOPs over zc-ring-x1 spsc queues

Two spsc queues in front of the ring: a request queue whose entries become SQEs, and a completion
queue the CQEs are copied into, so a caller never touches the ring itself.

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

## Ideas

Items not yet solid enough for `## Todo`. Triaged at the next opening: promote, fold into a
picked-up cycle, or drop.

## Bugs

_See [bugs.md](notes/bugs.md).

## Closed

The last cycle's finished record, moved here whole by its closing commit and deleted by the next
opening ([Cycle-record](AGENTS.md#cycle-record)).

# References
