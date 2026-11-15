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

## Ideas

Items not yet solid enough for `## Todo`. Triaged at the next opening: promote, fold into a
picked-up cycle, or drop.

## Bugs

_See [bugs.md](notes/bugs.md).

## Closed

The last cycle's finished record, moved here whole by its closing commit and deleted by the next
opening ([Cycle-record](AGENTS.md#cycle-record)).

# References
