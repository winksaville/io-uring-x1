//! `io-uring-x1`: experiments with Linux io_uring from its primitives.

use std::error::Error;
use std::process::ExitCode;

use io_uring_x1::ring::{Ring, nop};

/// Set up a ring, send four NOPs through it, and print each completion and the ring's indexes.
fn round_trip() -> Result<(), Box<dyn Error>> {
    let mut ring = Ring::new(8)?;
    println!("set up:    {:?}", ring.state());
    for user_data in 1..=4 {
        ring.push(&nop(user_data))?;
    }
    let consumed = ring.submit(4)?;
    println!("submitted: {consumed} NOP(s)");
    while let Some(c) = ring.pop() {
        println!("completed: {c:?}");
    }
    println!("reaped:    {:?}", ring.state());
    Ok(())
}

fn main() -> ExitCode {
    println!("io-uring-x1 {}", env!("CARGO_PKG_VERSION"));
    match round_trip() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("io-uring-x1: {e}");
            ExitCode::FAILURE
        }
    }
}
