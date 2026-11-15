//! `io-uring-x1`: experiments with Linux io_uring from its primitives.

use std::error::Error;
use std::process::ExitCode;
use std::thread;

use clap::Parser;
use io_uring_x1::driver::{self, Driver, NopCompletion, NopRequest, store};
use io_uring_x1::ring::{Depths, Ring, RingState};
use zc_ring_x1::{Pool, Ring as Spsc};
use zerocopy::IntoBytes;

/// The NOPs a round trip sends.
const NOPS: u64 = 4;

/// Slots in each spsc queue, a power of two.
const QUEUE_DEPTH: u32 = 8;

/// Send NOPs through an io_uring built from its primitives, by way of two zc-ring-x1 spsc queues,
/// printing each completion and the ring's indexes.
#[derive(Parser, Debug)]
// `version` gives `-V`, which `vc-x1 install` runs to read the version installed.
#[command(version, max_term_width = 100)]
struct Cli {
    /// The SQ's entries, rounded up to a power of two by the kernel
    #[arg(long, value_name = "N", default_value_t = Depths::DEFAULT_SQ)]
    sq_entries: u32,

    /// The CQ's entries, rounded up to a power of two and at least the SQ's [default: twice the
    /// SQ's]
    #[arg(long, value_name = "N")]
    cq_entries: Option<u32>,
}

/// Wait by spinning briefly, then yielding the thread, until `done` says to stop.
fn spin_then_yield(attempt: u32, done: impl Fn() -> bool) -> bool {
    if done() {
        return false;
    }
    if attempt < 100 {
        std::hint::spin_loop();
    } else {
        thread::yield_now();
    }
    true
}

/// Send the NOPs as requests into one spsc queue and read their completions from another, a
/// driver thread moving them through an io_uring of `depths`, and print each completion and the
/// ring's indexes.
fn round_trip(depths: Depths) -> Result<(), Box<dyn Error>> {
    let (mut lines, buf_size) = store(QUEUE_DEPTH, 2);
    let mut pool = Pool::init(lines.as_mut_slice().as_mut_bytes(), buf_size, 2)
        .map_err(|e| format!("the queues' pool: {e:?}"))?;
    let (mut requests, req_rx) = Spsc::init(&mut pool, driver::SLOT, QUEUE_DEPTH, 1)
        .map_err(|e| format!("the request queue: {e:?}"))?
        .split();
    let (cmp_tx, mut completions) = Spsc::init(&mut pool, driver::SLOT, QUEUE_DEPTH, 1)
        .map_err(|e| format!("the completion queue: {e:?}"))?
        .split();

    thread::scope(|s| {
        // The io_uring is made on the driver's thread, its pointers staying there, and the
        // driver runs until it has delivered every NOP's completion.
        let driver_thread = s.spawn(move || -> Result<RingState, String> {
            // The kernel's refusal is a bare errno, so the depths it refused are named beside it.
            let ring = Ring::with_depths(depths).map_err(|e| {
                format!(
                    "io_uring_setup refused {depths:?} (the CQ must be at least the SQ, and \
                     neither 0): {e}"
                )
            })?;
            println!("set up:    {:?}", ring.state());
            let mut driver = Driver::new(ring, req_rx, cmp_tx);
            let mut delivered = 0;
            while delivered < NOPS {
                let step = driver.step(true).map_err(|e| format!("the driver: {e}"))?;
                delivered += u64::from(step.completed);
                if step == Default::default() {
                    thread::yield_now();
                }
            }
            Ok(driver.ring_state())
        });

        // The caller: requests in, completions out, never touching the io_uring. A driver that
        // stops early ends both waits, and its result says why.
        let stopped = || driver_thread.is_finished();
        for user_data in 1..=NOPS {
            let Ok(mut slot) = requests
                .reserve_slot_with::<NopRequest>(|attempt| spin_then_yield(attempt, stopped))
            else {
                break;
            };
            *slot = NopRequest { user_data };
            slot.commit();
        }
        for _ in 0..NOPS {
            let Ok(c) = completions
                .reserve_slot_with::<NopCompletion>(|attempt| spin_then_yield(attempt, stopped))
            else {
                break;
            };
            println!("completed: {:?}", *c);
            c.release();
        }

        match driver_thread.join() {
            Ok(Ok(state)) => {
                println!("reaped:    {state:?}");
                Ok(())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err("the driver thread panicked".into()),
        }
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    println!("io-uring-x1 {}", env!("CARGO_PKG_VERSION"));
    let depths = Depths {
        sq: cli.sq_entries,
        cq: cli.cq_entries,
    };
    match round_trip(depths) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("io-uring-x1: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The depths parse as flags, the CQ's absent unless named.
    #[test]
    fn the_depths_are_flags() {
        let parse = |args: &[&str]| Cli::try_parse_from(args).map(|c| (c.sq_entries, c.cq_entries));
        assert_eq!(parse(&["x"]).ok(), Some((Depths::DEFAULT_SQ, None)));
        assert_eq!(parse(&["x", "--sq-entries", "2"]).ok(), Some((2, None)));
        assert_eq!(
            parse(&["x", "--sq-entries", "2", "--cq-entries", "16"]).ok(),
            Some((2, Some(16)))
        );
        assert!(
            parse(&["x", "--sq-entries", "-1"]).is_err(),
            "a depth is a u32"
        );
        let v = Cli::try_parse_from(["x", "-V"]).unwrap_err();
        assert_eq!(v.kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(v.to_string().contains(env!("CARGO_PKG_VERSION")), "{v}");
    }
}
