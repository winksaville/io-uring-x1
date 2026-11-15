//! An io_uring built from its primitives: `io_uring_setup`, the rings and the SQE array mapped by
//! `mmap`, the kernel's offsets resolved into pointers, and `io_uring_enter` to submit and wait.
//!
//! The SQ and CQ rings share one mapping, which the kernel reports by
//! `IORING_FEAT_SINGLE_MMAP` (every kernel since 5.4), and the SQE array is a second mapping.
//! A kernel without the feature is refused rather than given the two-mapping path.
//!
//! The head and tail protocol: each ring has one producer and one consumer, the producer
//! owning the tail and the consumer the head. We produce SQEs and consume CQEs, so we write
//! the SQ tail and the CQ head, each with `Release` after the entry it covers is written or
//! read, and read the kernel's SQ head and CQ tail with `Acquire`.

use std::ffi::c_void;
use std::fmt;
use std::io;
use std::mem::size_of;
use std::os::fd::OwnedFd;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use rustix::io_uring::{
    IORING_OFF_SQ_RING, IORING_OFF_SQES, IoringEnterFlags, IoringFeatureFlags, IoringOp,
    io_uring_cqe, io_uring_enter, io_uring_params, io_uring_setup, io_uring_sqe,
    io_uring_user_data,
};
use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};

/// One mapping: its address and length, unmapped by [`Ring`]'s drop.
struct Mapping {
    ptr: *mut c_void,
    len: usize,
}

impl Mapping {
    /// Map `len` bytes of the ring's fd at the kernel's `offset`, shared and populated.
    fn new(fd: &OwnedFd, len: usize, offset: u64) -> io::Result<Self> {
        // SAFETY: a fresh mapping at an address the kernel chooses, of the fd's region at
        // `offset`, which the kernel sized for `len` by the params `io_uring_setup` returned.
        let ptr = unsafe {
            mmap(
                null_mut(),
                len,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED | MapFlags::POPULATE,
                fd,
                offset,
            )
        }?;
        Ok(Self { ptr, len })
    }

    /// A pointer `off` bytes into the mapping, as a `T`.
    ///
    /// # Safety
    ///
    /// `off` plus the size of a `T` is within the mapping, and `off` is aligned for `T`.
    unsafe fn at<T>(&self, off: u32) -> *mut T {
        // SAFETY: the caller keeps `off` within the mapping.
        unsafe { self.ptr.cast::<u8>().add(off as usize).cast::<T>() }
    }
}

/// The ring's indexes and sizes at one moment, read from the shared mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingState {
    pub sq_entries: u32,
    pub sq_mask: u32,
    pub sq_head: u32,
    pub sq_tail: u32,
    pub cq_entries: u32,
    pub cq_mask: u32,
    pub cq_head: u32,
    pub cq_tail: u32,
}

/// A completion, copied out of its CQE before the CQ head moves past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completion {
    /// The `user_data` of the SQE it completes.
    pub user_data: u64,
    /// The operation's result: 0 or more on success, a negated errno on failure.
    pub res: i32,
    /// The CQE's flags, as the kernel's `IORING_CQE_F_*` bits.
    pub flags: u32,
}

/// The SQ has no free slot: every slot holds an SQE the kernel has not yet consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqFull;

impl fmt::Display for SqFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the submission queue is full")
    }
}

impl std::error::Error for SqFull {}

/// An `IORING_OP_NOP` SQE, which completes with `res` 0 and the `user_data` given.
pub fn nop(user_data: u64) -> io_uring_sqe {
    io_uring_sqe {
        opcode: IoringOp::Nop,
        user_data: io_uring_user_data::from_u64(user_data),
        ..Default::default()
    }
}

/// An io_uring instance: its fd, its two mappings, and the pointers into them.
pub struct Ring {
    // The pointers point into `rings` and `sqes`, which are unmapped by `Drop` before `fd`
    // closes, the fields dropping after `drop` runs.
    sq_head: *const AtomicU32,
    sq_tail: *const AtomicU32,
    sq_mask: u32,
    sq_entries: u32,
    sq_array: *mut u32,
    sqe_array: *mut io_uring_sqe,
    cq_head: *const AtomicU32,
    cq_tail: *const AtomicU32,
    cq_mask: u32,
    cq_entries: u32,
    cqe_array: *const io_uring_cqe,
    // SQEs published by the tail and not yet handed to the kernel by `io_uring_enter`.
    unsubmitted: u32,
    rings: Mapping,
    sqes: Mapping,
    fd: OwnedFd,
}

impl Ring {
    /// Set up a ring of at least `entries` SQEs, the kernel rounding up to a power of two and
    /// giving the CQ twice as many.
    pub fn new(entries: u32) -> io::Result<Self> {
        let mut p = io_uring_params::default();
        // SAFETY: `p` is zeroed, no flags asking for anything beyond the plain ring.
        let fd = unsafe { io_uring_setup(entries, &mut p) }?;

        if !p.features.contains(IoringFeatureFlags::SINGLE_MMAP) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "io_uring: the kernel lacks IORING_FEAT_SINGLE_MMAP",
            ));
        }

        // One mapping for both rings, sized for the larger: the SQ ends at its index array and
        // the CQ at its CQE array.
        let sq_len = p.sq_off.array as usize + p.sq_entries as usize * size_of::<u32>();
        let cq_len = p.cq_off.cqes as usize + p.cq_entries as usize * size_of::<io_uring_cqe>();
        let rings = Mapping::new(&fd, sq_len.max(cq_len), IORING_OFF_SQ_RING)?;
        let sqes = Mapping::new(
            &fd,
            p.sq_entries as usize * size_of::<io_uring_sqe>(),
            IORING_OFF_SQES,
        )?;

        // SAFETY: each offset is the kernel's, within `rings` by the length computed above, and
        // aligned for its field, the kernel laying the rings out as u32s and the CQEs at their
        // own alignment. The SQE array is the whole of `sqes`.
        let ring = unsafe {
            Self {
                sq_head: rings.at(p.sq_off.head),
                sq_tail: rings.at(p.sq_off.tail),
                sq_mask: *rings.at::<u32>(p.sq_off.ring_mask),
                sq_entries: *rings.at::<u32>(p.sq_off.ring_entries),
                sq_array: rings.at(p.sq_off.array),
                sqe_array: sqes.at(0),
                cq_head: rings.at(p.cq_off.head),
                cq_tail: rings.at(p.cq_off.tail),
                cq_mask: *rings.at::<u32>(p.cq_off.ring_mask),
                cq_entries: *rings.at::<u32>(p.cq_off.ring_entries),
                cqe_array: rings.at(p.cq_off.cqes),
                unsubmitted: 0,
                rings,
                sqes,
                fd,
            }
        };

        // The SQ's index array maps ring slot i to SQE i, once, so an SQE is addressed by the
        // slot it is published in.
        for i in 0..ring.sq_entries {
            // SAFETY: `i` is below the array's `sq_entries` entries, within `rings`.
            unsafe { ring.sq_array.add(i as usize).write(i) };
        }

        Ok(ring)
    }

    /// The indexes and sizes as the shared mapping holds them now.
    pub fn state(&self) -> RingState {
        // SAFETY: the index pointers are `self`'s, into `rings`.
        unsafe {
            RingState {
                sq_entries: self.sq_entries,
                sq_mask: self.sq_mask,
                sq_head: load(self.sq_head, Ordering::Acquire),
                sq_tail: load(self.sq_tail, Ordering::Acquire),
                cq_entries: self.cq_entries,
                cq_mask: self.cq_mask,
                cq_head: load(self.cq_head, Ordering::Acquire),
                cq_tail: load(self.cq_tail, Ordering::Acquire),
            }
        }
    }

    /// Publish `sqe` in the next SQ slot, for the next [`Ring::submit`] to hand to the kernel.
    pub fn push(&mut self, sqe: &io_uring_sqe) -> Result<(), SqFull> {
        // The tail is ours, so our own last store is current, and the head is the kernel's.
        // SAFETY: the index pointers are `self`'s, into `rings`.
        let (tail, head) = unsafe {
            (
                load(self.sq_tail, Ordering::Relaxed),
                load(self.sq_head, Ordering::Acquire),
            )
        };
        if tail.wrapping_sub(head) >= self.sq_entries {
            return Err(SqFull);
        }
        // SAFETY: the slot is below `sq_entries`, and free, the kernel having consumed what it
        // held, `head` having passed it.
        unsafe {
            self.sqe_array
                .add((tail & self.sq_mask) as usize)
                .write(*sqe)
        };
        // Release: the kernel that sees the new tail sees the SQE written before it.
        // SAFETY: `sq_tail` points into `rings`, mapped for `self`'s life.
        unsafe { (*self.sq_tail).store(tail.wrapping_add(1), Ordering::Release) };
        self.unsubmitted += 1;
        Ok(())
    }

    /// Hand the published SQEs to the kernel and wait until at least `want` completions are
    /// in the CQ, returning how many SQEs the kernel consumed.
    pub fn submit(&mut self, want: u32) -> io::Result<u32> {
        let flags = match want {
            0 => IoringEnterFlags::empty(),
            _ => IoringEnterFlags::GETEVENTS,
        };
        // SAFETY: every SQE counted in `unsubmitted` is fully written and published by the SQ
        // tail, and a NOP holds no pointer for the kernel to follow.
        let consumed = unsafe { io_uring_enter(&self.fd, self.unsubmitted, want, flags) }?;
        self.unsubmitted -= consumed;
        Ok(consumed)
    }

    /// Take the oldest completion from the CQ, `None` when it is empty.
    pub fn pop(&mut self) -> Option<Completion> {
        // The head is ours, and the tail is the kernel's: Acquire, so the CQE it covers is seen.
        // SAFETY: the index pointers are `self`'s, into `rings`.
        let (head, tail) = unsafe {
            (
                load(self.cq_head, Ordering::Relaxed),
                load(self.cq_tail, Ordering::Acquire),
            )
        };
        if head == tail {
            return None;
        }
        // SAFETY: the slot is below `cq_entries`, and filled, `tail` having passed it.
        let cqe = unsafe { &*self.cqe_array.add((head & self.cq_mask) as usize) };
        let completion = Completion {
            user_data: cqe.user_data.u64_(),
            res: cqe.res,
            flags: cqe.flags.bits(),
        };
        // Release: the kernel that sees the new head may reuse the slot, which we have read.
        // SAFETY: `cq_head` points into `rings`, mapped for `self`'s life.
        unsafe { (*self.cq_head).store(head.wrapping_add(1), Ordering::Release) };
        Some(completion)
    }
}

/// Load one of the ring's indexes.
///
/// # Safety
///
/// `p` is one of a live `Ring`'s index pointers, into its `rings` mapping.
unsafe fn load(p: *const AtomicU32, order: Ordering) -> u32 {
    // SAFETY: the caller's `p` is into a mapping that outlives the call.
    unsafe { (*p).load(order) }
}

impl Drop for Ring {
    fn drop(&mut self) {
        for m in [&self.sqes, &self.rings] {
            // SAFETY: each mapping is `Ring`'s own, unmapped once, here, and no pointer into it
            // outlives `self`. A failure leaves only a leaked mapping, so it is ignored.
            let _ = unsafe { munmap(m.ptr, m.len) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_is_set_up_with_the_kernels_sizes() -> io::Result<()> {
        let ring = Ring::new(8)?;
        assert_eq!(
            ring.state(),
            RingState {
                sq_entries: 8,
                sq_mask: 7,
                sq_head: 0,
                sq_tail: 0,
                cq_entries: 16,
                cq_mask: 15,
                cq_head: 0,
                cq_tail: 0,
            }
        );
        Ok(())
    }

    #[test]
    fn entries_round_up_to_a_power_of_two() -> io::Result<()> {
        let s = Ring::new(5)?.state();
        assert_eq!((s.sq_entries, s.cq_entries), (8, 16));
        Ok(())
    }

    /// Each NOP comes back once, with its own `user_data` and `res` 0, and the indexes all
    /// meet where the four entries left them.
    #[test]
    fn nops_round_trip_with_their_user_data() -> Result<(), Box<dyn std::error::Error>> {
        let mut ring = Ring::new(8)?;
        for ud in 100..104 {
            ring.push(&nop(ud))?;
        }
        assert_eq!(ring.submit(4)?, 4);
        let mut got: Vec<Completion> = std::iter::from_fn(|| ring.pop()).collect();
        got.sort_by_key(|c| c.user_data);
        let want: Vec<Completion> = (100..104)
            .map(|user_data| Completion {
                user_data,
                res: 0,
                flags: 0,
            })
            .collect();
        assert_eq!(got, want);
        let s = ring.state();
        assert_eq!((s.sq_head, s.sq_tail, s.cq_head, s.cq_tail), (4, 4, 4, 4));
        Ok(())
    }

    /// A full SQ refuses the next push, and a submit frees its slots. Three rounds of a full
    /// SQ carry the indexes past the slot count, so the masks are exercised.
    #[test]
    fn a_full_sq_refuses_until_submitted() -> Result<(), Box<dyn std::error::Error>> {
        let mut ring = Ring::new(4)?;
        let mut ud = 0;
        for _ in 0..3 {
            for _ in 0..4 {
                ring.push(&nop(ud))?;
                ud += 1;
            }
            assert_eq!(ring.push(&nop(ud)), Err(SqFull));
            assert_eq!(ring.submit(4)?, 4);
            assert_eq!(std::iter::from_fn(|| ring.pop()).count(), 4);
        }
        let s = ring.state();
        assert_eq!(
            (s.sq_head, s.sq_tail, s.cq_head, s.cq_tail),
            (12, 12, 12, 12)
        );
        Ok(())
    }
}
