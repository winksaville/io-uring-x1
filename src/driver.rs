//! NOPs carried over two zc-ring-x1 spsc queues: a request queue whose entries become SQEs, and a
//! completion queue the CQEs are copied into, so a caller never touches the io_uring itself.
//!
//! The [`Driver`] owns the io_uring, the request queue's consumer, and the completion queue's
//! producer, and each [`Driver::step`] moves what it can: requests into the SQ, the SQ into the
//! kernel, and CQEs into completions. The caller holds the other two endpoints, typically on
//! another thread, the spsc endpoints being `Send` and the io_uring staying on the driver's.
//!
//! Backpressure flows back through each queue: a request stays in its slot while the SQ is full
//! or the CQ could not take its completion, and a CQE stays in the CQ while the completion queue
//! is full, so nothing is dropped and nothing overflows.

use std::io;

use zc_ring_x1::spsc::v3::segment_size;
use zc_ring_x1::{Consumer, PoolHeader, Producer};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::ring::{Ring, RingState, nop};

/// Bytes per spsc slot: one cache line, zc-ring-x1's 16-byte slot header and up to 48 bytes of
/// message, room for either message type here.
pub const SLOT: u32 = 64;

/// One cache line of backing store, so a `Vec<Line>` is a line-aligned region of any size a pool
/// wants.
#[derive(Clone, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C, align(64))]
pub struct Line([u8; 64]);

/// A zeroed, line-aligned store for a pool of `segments` buffers, each holding an spsc segment of
/// `depth` slots of [`SLOT`] bytes, and that buffer size, for `Pool::init`.
pub fn store(depth: u32, segments: u32) -> (Vec<Line>, u32) {
    let seg = segment_size(SLOT, depth);
    let bytes = size_of::<PoolHeader>() as u64 + seg * segments as u64;
    (vec![Line([0; 64]); bytes.div_ceil(64) as usize], seg as u32)
}

/// A request for one NOP, carried in a request queue slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct NopRequest {
    /// Handed back unchanged in the NOP's completion.
    pub user_data: u64,
}

/// A NOP's completion, a CQE copied into a completion queue slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct NopCompletion {
    /// The request's `user_data`.
    pub user_data: u64,
    /// The CQE's result, 0 for a NOP.
    pub res: i32,
    /// The CQE's flags.
    pub flags: u32,
}

/// What one [`Driver::step`] moved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Step {
    /// Requests taken from the request queue into the SQ.
    pub queued: u32,
    /// SQEs the kernel consumed.
    pub submitted: u32,
    /// CQEs copied into the completion queue.
    pub completed: u32,
}

/// The io_uring and the queue endpoints it serves.
pub struct Driver<'a> {
    ring: Ring,
    requests: Consumer<'a>,
    completions: Producer<'a>,
    /// Requests in the SQ or the kernel whose CQEs are not yet copied out: kept at most the CQ's
    /// depth, so the kernel never has more completions than the CQ holds.
    in_flight: u32,
}

impl<'a> Driver<'a> {
    /// A driver of `ring`, taking requests from `requests` and giving completions to
    /// `completions`.
    pub fn new(ring: Ring, requests: Consumer<'a>, completions: Producer<'a>) -> Self {
        Self {
            ring,
            requests,
            completions,
            in_flight: 0,
        }
    }

    /// Requests taken and not yet completed.
    pub fn in_flight(&self) -> u32 {
        self.in_flight
    }

    /// The io_uring's indexes and sizes now.
    pub fn ring_state(&self) -> RingState {
        self.ring.state()
    }

    /// Move what can move: requests into the SQ, the SQ into the kernel, and CQEs into the
    /// completion queue. With `wait` and something in flight, block in the kernel until at least
    /// one completion is ready, so a driver with nothing new to do sleeps rather than spins.
    pub fn step(&mut self, wait: bool) -> io::Result<Step> {
        let mut step = Step::default();
        let cq_depth = self.ring.state().cq_entries;

        // Requests into the SQ, each released only once its SQE is published, so a full SQ
        // leaves the request in its slot to be read again.
        while self.in_flight < cq_depth {
            let Ok(req) = self.requests.reserve_slot_with::<NopRequest>(|_| false) else {
                break;
            };
            if self.ring.push(&nop(req.user_data)).is_err() {
                break; // dropping the read slot re-delivers it
            }
            req.release();
            self.in_flight += 1;
            step.queued += 1;
        }

        let want = u32::from(wait && self.in_flight > 0);
        step.submitted = self.ring.submit(want)?;

        // CQEs into completions, a completion slot reserved before the CQE is taken, so a full
        // completion queue leaves the CQE in the CQ.
        while let Ok(mut slot) = self
            .completions
            .reserve_slot_with::<NopCompletion>(|_| false)
        {
            let Some(c) = self.ring.pop() else {
                break; // dropping the write slot abandons the reservation
            };
            *slot = NopCompletion {
                user_data: c.user_data,
                res: c.res,
                flags: c.flags,
            };
            slot.commit();
            self.in_flight -= 1;
            step.completed += 1;
        }
        Ok(step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ring::Depths;
    use zc_ring_x1::{Pool, Ring as Spsc};

    /// A result's error as its `Debug` text: zc-ring-x1 is `no_std`, so its errors are not
    /// `std::error::Error`, and the tests report every error the same way.
    trait OrDbg<T> {
        fn or_dbg(self) -> Result<T, String>;
    }

    impl<T, E: std::fmt::Debug> OrDbg<T> for Result<T, E> {
        fn or_dbg(self) -> Result<T, String> {
            self.map_err(|e| format!("{e:?}"))
        }
    }

    /// NOPs pushed into the request queue come out of the completion queue, one each, with
    /// their `user_data` and `res` 0, on one thread, the driver stepped between.
    #[test]
    fn nops_cross_both_queues() -> Result<(), String> {
        const DEPTH: u32 = 8;
        let (mut lines, seg) = store(DEPTH, 2);
        let mut pool = Pool::init(lines.as_mut_slice().as_mut_bytes(), seg, 2).or_dbg()?;
        let (mut req_tx, req_rx) = Spsc::init(&mut pool, SLOT, DEPTH, 1).or_dbg()?.split();
        let (cmp_tx, mut cmp_rx) = Spsc::init(&mut pool, SLOT, DEPTH, 1).or_dbg()?.split();
        let mut driver = Driver::new(Ring::new(4).or_dbg()?, req_rx, cmp_tx);

        for user_data in 1..=6 {
            let mut slot = req_tx.reserve_slot_with::<NopRequest>(|_| false).or_dbg()?;
            *slot = NopRequest { user_data };
            slot.commit();
        }
        let mut got = Vec::new();
        while got.len() < 6 {
            driver.step(true).or_dbg()?;
            while let Ok(c) = cmp_rx.reserve_slot_with::<NopCompletion>(|_| false) {
                got.push(*c);
                c.release();
            }
        }
        got.sort_by_key(|c| c.user_data);
        let want: Vec<NopCompletion> = (1..=6)
            .map(|user_data| NopCompletion {
                user_data,
                res: 0,
                flags: 0,
            })
            .collect();
        assert_eq!(got, want);
        assert_eq!(driver.in_flight(), 0);
        Ok(())
    }

    /// A completion queue too small for what is in flight holds the rest in the CQ, and the
    /// requests the CQ could not take stay queued, until the caller drains: nothing is lost.
    #[test]
    fn a_full_completion_queue_holds_cqes_back() -> Result<(), String> {
        let (mut lines, seg) = store(8, 2);
        let mut pool = Pool::init(lines.as_mut_slice().as_mut_bytes(), seg, 2).or_dbg()?;
        let (mut req_tx, req_rx) = Spsc::init(&mut pool, SLOT, 8, 1).or_dbg()?.split();
        let (cmp_tx, mut cmp_rx) = Spsc::init(&mut pool, SLOT, 2, 1).or_dbg()?.split();
        let ring = Ring::with_depths(Depths { sq: 4, cq: Some(4) }).or_dbg()?;
        let mut driver = Driver::new(ring, req_rx, cmp_tx);

        for user_data in 0..8 {
            let mut slot = req_tx.reserve_slot_with::<NopRequest>(|_| false).or_dbg()?;
            *slot = NopRequest { user_data };
            slot.commit();
        }
        // The CQ holds 4, so 4 go in flight, and the completion queue takes 2 of their CQEs.
        let s = driver.step(true).or_dbg()?;
        assert_eq!((s.queued, s.completed), (4, 2));
        assert_eq!(driver.in_flight(), 2);
        // With the completion queue still full, nothing more completes, and 2 more requests
        // fill the CQ's room.
        let s = driver.step(false).or_dbg()?;
        assert_eq!((s.queued, s.completed), (2, 0));
        let mut seen = 0;
        while seen < 8 {
            while let Ok(c) = cmp_rx.reserve_slot_with::<NopCompletion>(|_| false) {
                assert_eq!(c.res, 0);
                c.release();
                seen += 1;
            }
            driver.step(true).or_dbg()?;
        }
        assert_eq!(driver.in_flight(), 0);
        Ok(())
    }
}
