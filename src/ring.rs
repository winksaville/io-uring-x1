//! An io_uring built from its primitives: `io_uring_setup`, the rings and the SQE array mapped by
//! `mmap`, and the kernel's offsets resolved into pointers.
//!
//! The SQ and CQ rings share one mapping, which the kernel reports by
//! `IORING_FEAT_SINGLE_MMAP` (every kernel since 5.4), and the SQE array is a second mapping.
//! A kernel without the feature is refused rather than given the two-mapping path.

use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::os::fd::OwnedFd;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use rustix::io_uring::{
    IORING_OFF_SQ_RING, IORING_OFF_SQES, IoringFeatureFlags, io_uring_cqe, io_uring_params,
    io_uring_setup, io_uring_sqe,
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

/// An io_uring instance: its fd, its two mappings, and the pointers into them.
pub struct Ring {
    // The pointers point into `rings` and `sqes`, which are unmapped by `Drop` before `fd`
    // closes, the fields dropping after `drop` runs.
    sq_head: *const AtomicU32,
    sq_tail: *const AtomicU32,
    sq_mask: u32,
    sq_entries: u32,
    sq_array: *mut u32,
    cq_head: *const AtomicU32,
    cq_tail: *const AtomicU32,
    cq_mask: u32,
    cq_entries: u32,
    rings: Mapping,
    sqes: Mapping,
    _fd: OwnedFd,
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
        // aligned for its field, the kernel laying the rings out as u32s.
        let ring = unsafe {
            Self {
                sq_head: rings.at(p.sq_off.head),
                sq_tail: rings.at(p.sq_off.tail),
                sq_mask: *rings.at::<u32>(p.sq_off.ring_mask),
                sq_entries: *rings.at::<u32>(p.sq_off.ring_entries),
                sq_array: rings.at(p.sq_off.array),
                cq_head: rings.at(p.cq_off.head),
                cq_tail: rings.at(p.cq_off.tail),
                cq_mask: *rings.at::<u32>(p.cq_off.ring_mask),
                cq_entries: *rings.at::<u32>(p.cq_off.ring_entries),
                rings,
                sqes,
                _fd: fd,
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
        // SAFETY: each pointer is into `rings`, mapped for `self`'s life, and the kernel's
        // indexes are read atomically.
        let load = |p: *const AtomicU32| unsafe { (*p).load(Ordering::Acquire) };
        RingState {
            sq_entries: self.sq_entries,
            sq_mask: self.sq_mask,
            sq_head: load(self.sq_head),
            sq_tail: load(self.sq_tail),
            cq_entries: self.cq_entries,
            cq_mask: self.cq_mask,
            cq_head: load(self.cq_head),
            cq_tail: load(self.cq_tail),
        }
    }
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
}
