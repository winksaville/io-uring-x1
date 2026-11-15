//! The workspace's one binary, run by `cargo run`.
//!
//! The binaries are the workspace members' bin targets, as `cargo
//! metadata` reports them, the caller's own package left out, so a
//! workspace with one binary beside its xtask never asks which.

use std::path::Path;
use std::process::Command;

/// Run `bin`, or the workspace's one binary, every package named `skip`
/// left out, by `cargo run`, `args` passed through. Says nothing of its
/// own, the binary's output being all.
pub fn run(root: &Path, bin: Option<&str>, args: &[String], skip: &str) -> Result<String, String> {
    let found;
    let bin = match bin {
        Some(bin) => bin,
        None => {
            found = binaries(&metadata(root)?, skip);
            the_one(&found)?
        }
    };
    let status = Command::new(crate::cargo())
        .args(["run", "--bin", bin, "--"])
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("run: cargo run did not run: {e}"))?;
    match status.success() {
        true => Ok(String::new()),
        false => Err(format!("run: {bin} failed: {status}")),
    }
}

/// The workspace's `cargo metadata --no-deps`, parsed.
fn metadata(root: &Path) -> Result<serde_json::Value, String> {
    let out = Command::new(crate::cargo())
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("run: cargo metadata did not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "run: cargo metadata failed: {}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("run: cargo metadata's JSON does not parse: {e}"))
}

/// The bin targets of the workspace's packages in `cargo metadata
/// --no-deps` JSON, which lists the members alone, every package named
/// `skip` left out.
pub fn binaries(metadata: &serde_json::Value, skip: &str) -> Vec<String> {
    let empty = Vec::new();
    let packages = metadata["packages"].as_array().unwrap_or(&empty); // OK: no packages, no binaries
    packages
        .iter()
        .filter(|p| p["name"] != skip)
        .flat_map(|p| p["targets"].as_array().unwrap_or(&empty)) // OK: no targets, no binaries
        .filter(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|k| k == "bin"))
        })
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect()
}

/// The one binary there is to run, or why there is not one.
pub fn the_one(binaries: &[String]) -> Result<&str, String> {
    match binaries {
        [one] => Ok(one),
        [] => Err("run: the workspace has no binary to run".to_string()),
        several => Err(format!(
            "run: the workspace has {} binaries, name one with --bin: {}",
            several.len(),
            several.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The binaries are the members' bin targets, the skipped package's
    /// and every lib left out, and one of them is the one to run.
    #[test]
    fn run_finds_the_one_binary() {
        let metadata = serde_json::json!({"packages": [
            {"name": "io-uring-x1", "targets": [
                {"name": "io_uring_x1", "kind": ["lib"]},
                {"name": "io-uring-x1", "kind": ["bin"]},
            ]},
            {"name": "xtask", "targets": [{"name": "xtask", "kind": ["bin"]}]},
        ]});
        let found = binaries(&metadata, "xtask");
        assert_eq!(found, ["io-uring-x1"]);
        assert_eq!(the_one(&found), Ok("io-uring-x1"));
        let both = binaries(&metadata, "none");
        assert_eq!(both, ["io-uring-x1", "xtask"]);
        let e = the_one(&both).unwrap_err();
        assert!(
            e.contains("2 binaries") && e.contains("io-uring-x1, xtask"),
            "{e}"
        );
        assert!(the_one(&[]).unwrap_err().contains("no binary"));
        assert_eq!(
            binaries(&serde_json::json!({}), "xtask"),
            Vec::<String>::new()
        );
    }

    /// The workspace's metadata, read for real: leaving out a package
    /// that has binaries drops exactly its binaries, whichever workspace
    /// the library is tested in.
    #[test]
    fn the_real_metadata_skips_one_package() -> Result<(), String> {
        let metadata = metadata(&crate::workspace_root()?)?;
        let all = binaries(&metadata, "");
        let one = |p: &serde_json::Value| binaries(&serde_json::json!({ "packages": [p] }), "");
        let Some(package) = metadata["packages"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| !one(p).is_empty()))
        else {
            return Ok(()); // a workspace with no binary has nothing to skip
        };
        let Some(name) = package["name"].as_str() else {
            return Err(format!("a package with no name: {package}"));
        };
        let rest = binaries(&metadata, name);
        assert_eq!(
            rest.len() + one(package).len(),
            all.len(),
            "{all:?} {rest:?}"
        );
        assert!(one(package).iter().all(|b| !rest.contains(b)), "{rest:?}");
        Ok(())
    }
}
