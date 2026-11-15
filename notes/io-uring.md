# io_uring from its primitives

What the ring in `src/ring.rs` and the driver in `src/driver.rs` rest on: the kernel's layout, the
index protocol, the lifetimes of what a request touches, and how zc-ring-x1's shared memory fits.
Made by the cycle `feat: issue IORING_OP_NOP via io_uring primitives`, whose record is its
landmark's `TODO.md > ## Closed`.

## Layout

`io_uring_setup` returns a fd and, in `io_uring_params`, the offset of every field the application
needs, `sq_off` and `cq_off`. Three arrays live in memory mapped from the fd:

- The SQ ring: its head, tail, mask, entry count, and an array of `u32` indexes into the SQE array,
  an indirection `IORING_SETUP_NO_SQARRAY` (6.6) removes. `Ring::new` fills it once, slot i naming
  SQE i.
- The SQE array: `io_uring_sqe`, 64 bytes each (128 with `IORING_SETUP_SQE128`), its own mapping at
  `IORING_OFF_SQES`.
- The CQ ring: its head, tail, mask, entry count, and the `io_uring_cqe` array inline, 16 bytes
  each (32 with `IORING_SETUP_CQE32`).

With `IORING_FEAT_SINGLE_MMAP` (5.4) both rings share one mapping, which `Ring` requires.

- The entries are uapi structs, used as rustix gives them.
- The ring headers are not: their layout is the kernel's to change, which is why the offsets
  exist, so `Ring` holds a pointer per field, as io_uring(7)'s example and liburing do. A
  `#[repr(C)]` mirror would be no faster, a field access costing one load either way.
- The pointers point into memory `Ring` owns, which a Rust reference cannot express, so they are
  raw, private, and used only by `Ring`'s methods, and `Drop` unmaps before the fd closes.

## The index protocol

Each ring has one producer and one consumer, and each index one writer.

| Index | Writer | Our load | Our store |
|---|---|---|---|
| SQ tail | us | `Relaxed`, our own last store | `Release`, after the SQE is written |
| SQ head | kernel | `Acquire` | none |
| CQ tail | kernel | `Acquire`, so the CQE is seen | none |
| CQ head | us | `Relaxed` | `Release`, after the CQE is copied out |

- The indexes run free and wrap, a slot being the index masked, so a full SQ is `tail - head` equal
  to its entries.
- `io_uring_enter` may consume fewer SQEs than offered, so the published-not-entered count is kept
  and `submit` returns what was taken.
- The kernel never holds more completions than the CQ has room for when the requests in flight
  stay at most the CQ's depth, which the driver keeps, so no CQ overflow arises.

## What a request touches, and for how long

Only the rings and the SQE array are shared by `mmap`. What an SQE points at is ordinary memory of
the submitting process, which the kernel reaches through its address space, by copying to or from
user addresses or by pinning pages for direct I/O.

| Item | Where it lives | Valid until |
|---|---|---|
| SQE | shared mapping | submitted, the SQ head past it |
| Structures the SQE points at, `iovec`, `timespec`, `msghdr` | the process's memory | submitted, with `IORING_FEAT_SUBMIT_STABLE` (5.5) |
| Data buffers | the process's memory | the CQE arrives |
| CQE | shared mapping | the CQ head is advanced past it |

- Registered buffers (`IORING_REGISTER_BUFFERS`) are pinned once and named by `buf_index`, still
  the process's own memory. Provided buffer rings may be the kernel's, mapped, or the process's,
  registered.
- A data buffer must not be touched by anyone from `push` until its CQE. A borrow cannot enforce
  that, since safe code may `mem::forget` a guard while the kernel still writes, so a buffer's
  ownership moves into the request and comes back with the completion, as tokio-uring does.

## Zero-copy across processes

zc-ring-x1's pool and queues live in a `MAP_SHARED` region, a file in `/dev/shm`, so actors in
different processes share buffers and queues with no copy. io_uring keeps that:

- The kernel does not care whether a buffer's page is private or shared, and tmpfs is the best case.
- An SQE's address is read in the submitter's address space, so pool references stay offsets,
  never pointers, and become the local address when the SQE is built.
- One ring per process, or per thread, and cross-process traffic over the spsc and mpsc queues: a
  ring shared between processes reads an SQE's pointers in whichever address space submits, which
  `IORING_SETUP_SQPOLL` makes the creator's.
- Ordering carries across: the kernel's release of the CQ tail, our `Acquire` of it, then the spsc
  commit's `Release` and the other process's `Acquire`, so data the kernel wrote is seen by the
  consuming actor.
- `MAP_SHARED` never copies on write, so a fork's copy-on-write hazard with pinned buffers does not
  arise.
- We think shmem and hugetlbfs pages can be registered as fixed buffers and a `MAP_SHARED` mapping
  of a regular file cannot, recent kernels refusing long-term pins of file-backed memory. A test
  settles it when registration comes.
- Registered buffers count against `RLIMIT_MEMLOCK`, the ring's own memory not since 5.12.

## Where the speed is

Not in the ring's shape but in what crosses into the kernel and the shared lines:

- Cache the kernel's SQ head and reload it with `Acquire` only when the SQ looks full, as liburing
  does, where `push` now loads it every time.
- Batch SQEs per `io_uring_enter`, or enter less with `IORING_SETUP_SQPOLL`.
- `IORING_SETUP_NO_SQARRAY` drops the index array.
- Copy small fixed records, a CQE's 16 bytes, rather than hold a guard on the slot, and never copy
  payloads, moving their ownership instead.
