//! `io-uring-x1`: experiments with Linux io_uring from its primitives.

use std::error::Error;
use std::process::ExitCode;

use clap::Parser;
use io_uring_x1::ring::{Depths, Ring, SqFull, nop};

/// The NOPs a round trip sends.
const NOPS: u64 = 4;

/// Send NOPs through an io_uring built from its primitives, printing each completion and the
/// ring's indexes.
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

/// Set up a ring of `depths`, send the NOPs through it in batches the SQ holds, and print each
/// completion and the ring's indexes.
fn round_trip(depths: Depths) -> Result<(), Box<dyn Error>> {
    // The kernel's refusal is a bare errno, so the depths it refused are named beside it.
    let mut ring = Ring::with_depths(depths).map_err(|e| {
        format!(
            "io_uring_setup refused {depths:?} (the CQ must be at least the SQ, and neither 0): {e}"
        )
    })?;
    println!("set up:    {:?}", ring.state());
    let mut next = 1;
    while next <= NOPS {
        let mut batch = 0;
        while next <= NOPS {
            match ring.push(&nop(next)) {
                Ok(()) => {
                    next += 1;
                    batch += 1;
                }
                Err(SqFull) => break,
            }
        }
        let consumed = ring.submit(batch)?;
        println!("submitted: {consumed} NOP(s)");
        while let Some(c) = ring.pop() {
            println!("completed: {c:?}");
        }
    }
    println!("reaped:    {:?}", ring.state());
    Ok(())
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
