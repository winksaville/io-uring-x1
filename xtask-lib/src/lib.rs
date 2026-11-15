//! `xtask-lib`: the tasks behind `cargo xtask`, as a package of their
//! own, so any project's `xtask` binary can depend on them: each task a
//! module, and that binary's `main` the arguments and the dispatch.
//!
//! - [`version`]: the version in `Cargo.toml`, and its bump by the suffix
//!   scheme, `Cargo.lock` refreshed in the same step.
//! - [`validate`]: the workspace validated, read-only, by the commands the
//!   caller gives, [`validate::commands`] being the default.
//! - [`run`]: the workspace's one binary run by `cargo run`, the caller's
//!   own package left out.
//! - [`completions`]: a bash function completing `cargo xtask` by asking
//!   the xtask of the repo the shell is in.
//!
//! Nothing here is compiled for one workspace: the root is found at
//! runtime by [`workspace_root`], and what is a project's own, the
//! validation commands and the package `run` leaves out, is passed in.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

pub mod completions;
pub mod run;
pub mod validate;
pub mod version;

/// The cargo that runs this task, `CARGO`, absent only when the binary
/// is run by hand.
pub fn cargo() -> OsString {
    match std::env::var_os("CARGO") {
        Some(cargo) => cargo,
        None => "cargo".into(),
    }
}

/// The root of the workspace the task is run in, as cargo finds it from
/// the current directory, the directory of the workspace's `Cargo.toml`.
pub fn workspace_root() -> Result<PathBuf, String> {
    let out = Command::new(cargo())
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .map_err(|e| format!("cargo locate-project did not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo locate-project failed: {}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let manifest = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim_end());
    manifest
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{} has no directory", manifest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The root holds the workspace's manifest, a `[workspace]` one, so
    /// it is the workspace's and not this package's.
    #[test]
    fn the_root_is_the_workspaces() -> Result<(), String> {
        let root = workspace_root()?;
        let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
            .map_err(|e| format!("{}: {e}", root.display()))?;
        assert!(manifest.contains("[workspace]"), "{}", root.display());
        Ok(())
    }
}
