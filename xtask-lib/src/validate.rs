//! The workspace validated, read-only, by a list of cargo commands.
//!
//! Nothing is written, so a validation can run at any point without
//! changing what was reviewed, and formatting is `cargo fmt`'s, run while
//! working.

use std::path::Path;
use std::process::Command;

/// The default cargo commands, in order, each one's arguments: a format
/// check, clippy over every target with warnings denied, and every test,
/// or with `fast` the libraries' and binaries' tests alone.
///
/// None of them writes a source file: the format is checked, never
/// applied.
pub fn commands(fast: bool) -> Vec<Vec<&'static str>> {
    let tests = vec!["test", "--workspace", "--lib", "--bins"];
    match fast {
        true => vec![tests],
        false => vec![
            vec!["fmt", "--all", "--check"],
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            vec!["test", "--workspace"],
        ],
    }
}

/// Run `commands` by cargo in the workspace root, each announced before
/// it runs, its output its own, and the first failure stopping the run.
/// `label` names the set in the closing line, `full` or `fast`.
pub fn validate(root: &Path, commands: &[Vec<&str>], label: &str) -> Result<String, String> {
    let n = commands.len();
    for (i, args) in commands.iter().enumerate() {
        let shown = format!("[{}/{n}] cargo {}", i + 1, args.join(" "));
        println!("validate: {shown}");
        let status = Command::new(crate::cargo())
            .args(args)
            .current_dir(root)
            .status()
            .map_err(|e| format!("validate: {shown} did not run: {e}"))?;
        if !status.success() {
            return Err(format!("validate: {shown} failed: {status}"));
        }
    }
    Ok(format!("validate: {n} command(s) passed ({label})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validation is read-only, the format checked and never applied,
    /// and `fast` is the tests alone, the libraries' included, which
    /// `cargo test --bins` skipped.
    #[test]
    fn validation_checks_and_never_writes() {
        let full = commands(false);
        assert_eq!(full[0], ["fmt", "--all", "--check"]);
        assert!(full.iter().all(|c| c[0] != "fmt" || c.contains(&"--check")));
        assert!(
            full.iter()
                .any(|c| c[0] == "clippy" && c.ends_with(&["-D", "warnings"]))
        );
        assert!(full.iter().any(|c| c[0] == "test"));
        let fast = commands(true);
        assert_eq!(fast, [["test", "--workspace", "--lib", "--bins"]]);
    }
}
