//! `io-uring-x1`: experiments with Linux io_uring from its primitives.

use std::process::ExitCode;

use io_uring_x1::ring::Ring;

fn main() -> ExitCode {
    println!("io-uring-x1 {}", env!("CARGO_PKG_VERSION"));
    match Ring::new(8) {
        Ok(ring) => {
            println!("{:?}", ring.state());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("io_uring setup failed: {e}");
            ExitCode::FAILURE
        }
    }
}
