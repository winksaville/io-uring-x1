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

_No cycle currently in progress._

## Waiting

Important work that cannot start yet. Each entry names what it waits on, in a form that can be
checked, and the rank it takes in `## Todo` once unblocked. Every opening checks each condition and
promotes what is met ([Opening](AGENTS.md#opening)).

## Todo

Entries are in priority order, the first highest, and reprioritizing is moving an entry. Each is a
`###` heading, so a citation is a link to its anchor. Long-tail entries live in
[todo-backlog.md](notes/todo-backlog.md). Use the [Prose form](agent-data/prose.md#prose-form).

### feat: issue IORING_OP_NOP using io_uring primatives

Documentation io_uring: https://man7.org/linux/man-pages/man7/io_uring.7.html
Dos for io_uring_enter and the opcodes: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html

## Ideas

Items not yet solid enough for `## Todo`. Triaged at the next opening: promote, fold into a
picked-up cycle, or drop.

## Bugs

_See [bugs.md](notes/bugs.md).

## Closed

The last cycle's finished record, moved here whole by its closing commit and deleted by the next
opening ([Cycle-record](AGENTS.md#cycle-record)).

### feat: configure repo for vc

#### Problem

Add cargo task using the model in ../vc-x1, this should be a single-step cycle. The workspace has
the agent-files but no Cargo project, so `vc-x1 validate`, `build`, and `install` have nothing to
run, and the version lives in a generic `version.toml`.

#### Solution

A Cargo workspace modelled on vc-x1's. The `io-uring-x1` binary package, a stub printing its
version, and the `xtask` package, vc-x1's own `cargo xtask` with its `version`, `bump`, and
`completions` tasks, reached by the `xtask` alias in `.cargo/config.toml`. The version-of-record
moves from `version.toml`, removed, into `Cargo.toml`, still `0.1.0`, the bare version of a
single-step cycle. `.vc-config.md` gains `[validate]`, `[build]`, `[install]`, `[profile.*]`, and
`[version]` as vc-x1 has them, the dev install named `io-uring-x1-dev`. The adoption of the
agent-files, left uncommitted on `@` by the setup, lands with this cycle, and the README says what
the project is and what the tasks are.

#### Acceptance check

`vc-x1 validate` passes, `cargo xtask version` prints `0.1.0`, `vc-x1 validate-config` finds no
problem, and `vc-x1 install --profile dev` installs `io-uring-x1-dev`, which prints
`io-uring-x1 0.1.0`.

#### Deliberation

- The setup left `## In Progress` holding a `### _None_` scaffold, which `cycle --open` reads as a
  cycle in progress, and a pre-made bookmark it refuses to reuse. The scaffold became the
  between-cycles line `_No cycle currently in progress._` and the local, unpushed bookmark was
  deleted, so `cycle --open` made both afresh on the same `@`.
- `.vc-config.md` as `vc-x1 init` wrote it held a commented `[build]` table, and adding vc-x1's
  `[build]` made a duplicate table that `vc-x1 validate-config` did not report. The two are one
  table now.
- The xtask is vc-x1's as is, less its `clippy::disallowed_methods` allowance, a lint this
  workspace does not set.

# References
