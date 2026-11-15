# Todo and cycle record

This file contains near term tasks with a short description and reference links to more details.
Its shape is [Todo format](agent-data/notes.md#todo-format).

## Continuation notes

Where the agent was, for the agent that comes next. Ephemeral, never a record, read first at
acquaint, acted on, and reset to `_None._` by the reader.

- Check the Land: `main` at the single-step commit `build: pin zc-ring-x1 by git, not the sibling
  path` in both repos and on origin, the bookmark `build-pin-zc-ring-x1-by-git-not-the-sibling-path`
  gone locally and on origin, and `io-uring-x1 -V` reporting `0.2.1`.
- Messages: the `m-12 m-13 m-14` batch in `../vc-x1-messages` was committed locally, `m-12` closed;
  check it reached `main@origin`. `m-13` is left open for vc-x1's line linking its landed cycle, our
  `done` already in it, and is closed once that line is there.

## In Progress

A cycle's record has one home at a time, and while the cycle runs this is it. The block's shape is
the specimen in [cycle-model.md](agent-data/cycle-model.md), and the rules are in [The In Progress
block](agent-data/notes.md#the-in-progress-block).

_No cycle currently in progress._

## Waiting

Important work that cannot start yet. Each entry names what it waits on, in a form that can be
checked, and the rank it takes in `## Todo` once unblocked. Every opening checks each condition and
promotes what is met ([Opening](AGENTS.md#opening)).

_None._

## Todo

Entries are in priority order, the first highest, and reprioritizing is moving an entry. Each is a
`###` heading, so a citation is a link to its anchor. Long-tail entries live in
[todo-backlog.md](notes/todo-backlog.md). Use the [Prose form](agent-data/prose.md#prose-form).

### feat: build and install by cargo xtask

`[build]`, `[install]`, and `[profile.*]` in `.vc-config.md` are the project's knowledge in vc-x1's
config, as `[validate]` was before `cargo xtask validate`. Move them into xtask, `cargo xtask install
--profile dev` for one, with a `custom.md` override where the agent-files name `vc-x1 install`.

### feat: io-uring-x1 completes its own flags

`io-uring-x1` has a CLI, `--sq-entries` and `--cq-entries`, and no completion. Give it clap's
`CompleteEnv`, as xtask has, so `source <(COMPLETE=bash io-uring-x1)` registers it, and
`io-uring-x1-dev` its own the same way. The depths complete to the powers of two up to the kernel's
limits, 32768 for the SQ and 65536 for the CQ, the values the kernel rounds to. It needs
`clap_complete` with `unstable-dynamic`, as xtask's does.

### feat: cx run hands completion to the binary

Once the binary's arguments have begun, `cargo xtask run` completes nothing, where it could ask the
binary. In `xtask-lib`'s `run`, run the built `target/debug/<bin>` with `COMPLETE=bash`, the words
after `run` given with the binary's name first, and relay its answer, so `cx run --sq-entries <tab>`
offers the depths. A tab never builds: no binary built, no completion, and one a build behind
completes a build behind. Waits on `feat: io-uring-x1 completes its own flags`, and serves any
binary with `CompleteEnv`.

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

### build: pin zc-ring-x1 by git, not the sibling path

#### Problem

`zc-ring-x1` is a path dependency on `../zc-ring-x1`, so a build takes whatever that working copy
holds and a clone without the sibling does not build. Switch to `git =
"https://github.com/winksaville/zc-ring-x1.git"` at `tag = "v0.19.7"` (`8cc34343`), the latest tag,
zc-ring-x1 having tagged its versions in reply to `m-12` in `../vc-x1-messages`. `core::error::Error`
for zc-ring-x1's error types, so `?` reaches `Box<dyn Error>`, was not asked, and is a later message
if it is wanted.

#### Solution

`Cargo.toml` names zc-ring-x1 by `git = "https://github.com/winksaville/zc-ring-x1.git"` and `tag =
"v0.19.7"`, the `path` dropped, and `Cargo.lock` records the git source at `8cc34343`, so every
build takes the tagged commit whatever `../zc-ring-x1` holds.

#### Acceptance check

`Cargo.lock` gives zc-ring-x1 0.19.7 the source
`git+https://github.com/winksaville/zc-ring-x1.git?tag=v0.19.7#8cc34343...`, `cargo xtask validate`
passes, and a clone of the cycle's tip into a directory with no `zc-ring-x1` sibling builds with
`cargo build --locked` and passes `cargo test`.

#### Deliberation

- The tag, not a `rev`: zc-ring-x1 tags every Land now, so a tag names the version as the manifest
  does and a later move is a one-word edit. `Cargo.lock` holds the commit, so a moved tag cannot
  change a `--locked` build silently.
- v0.19.7, not v0.19.5: the working copy's `Cargo.lock` already builds against 0.19.7, taken from
  the sibling path, so the pin keeps the code that builds today.
- `version = "0.19.7"` is kept beside the git source, so Cargo checks the tagged commit's manifest
  against it.
- Single-step, not a ladder: the change is one manifest line and the lockfile it implies, so the
  planned work rung was dropped and the cycle is one commit.

# References
