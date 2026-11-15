//! `cargo xtask`: the project's own tasks, modelled on vc-x1's, their
//! work in the `xtask-lib` library and the arguments and dispatch here.
//!
//! - `cargo xtask version` prints the version in `Cargo.toml`.
//! - `cargo xtask bump <next|bare|VERSION>` sets it: `next` adds one to
//!   the suffix's last number, `0.85.22-8` to `0.85.22-9`, and gives a
//!   bare version its first, `-0`, `bare` drops the suffix, a
//!   close-out's version, and anything else is the version as written.
//!   `Cargo.lock` is refreshed in the same step, since the build is
//!   `--locked`.
//! - `cargo xtask validate [--fast]` validates the workspace, read-only:
//!   `cargo fmt --check`, clippy with warnings denied, and the tests,
//!   or with `--fast` the tests alone. Formatting is `cargo fmt`'s, run
//!   while working, never validation's.
//! - `cargo xtask run [--bin NAME] [-- ARGS]` runs the workspace's one
//!   binary, xtask's own left out, found by `cargo metadata`, so
//!   `cargo run`'s question of which binary is answered when there is
//!   only one to answer it with.
//! - `cargo xtask completions [NAME...]` prints a bash function that
//!   completes `cargo xtask` and each NAME, a shell alias for it, by
//!   asking the xtask of the repo the shell is in.
//!
//! The arguments are clap's, whose completion engine answers when
//! `COMPLETE` is set.

use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use clap_complete::engine::{ArgValueCandidates, CompletionCandidate};
use xtask_lib::version::Bump;
use xtask_lib::{completions, run, validate, version, workspace_root};

/// The project's own tasks.
#[derive(Parser, Debug)]
#[command(name = "xtask", bin_name = "cargo xtask", max_term_width = 80)]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

/// A task.
#[derive(Subcommand, Debug)]
enum Task {
    /// Print the version in Cargo.toml
    Version,

    /// Set the version in Cargo.toml and refresh Cargo.lock
    #[command(long_about = "\
        Set the package version in Cargo.toml, its comments and layout kept,\n\
        and refresh Cargo.lock in the same step, the build being --locked.\n\n\
        `next` adds one to the suffix's last number, 0.85.22-8 to 0.85.22-9,\n\
        and gives a bare version its first, -0. `bare` drops the suffix, a\n\
        close-out's version. Anything else is the version as written, X.Y.Z\n\
        or X.Y.Z-<suffix>, with one `-` at most and no `+`.")]
    Bump {
        /// `next`, `bare`, or the version as written
        #[arg(value_name = "next|bare|VERSION", add = ArgValueCandidates::new(bump_candidates))]
        to: String,
    },

    /// Validate the workspace, read-only: format check, clippy, tests
    #[command(long_about = "\
        Validate the workspace, read-only, each command in order and the\n\
        first failure stopping the run: `cargo fmt --check`, clippy over\n\
        every target with warnings denied, and every test. Nothing is\n\
        written, so a validation can run at any point without changing what\n\
        was reviewed, and formatting is `cargo fmt`'s, run while working.\n\n\
        --fast runs the tests of the libraries and binaries alone, for while\n\
        a review iterates.")]
    Validate {
        /// The tests alone, without the format check and clippy
        #[arg(long)]
        fast: bool,
    },

    /// Run the workspace's one binary, passing ARGS through
    #[command(long_about = "\
        Run the workspace's binary by `cargo run --bin`, ARGS passed through\n\
        as `cargo run` passes them: the first argument that is not run's own\n\
        and everything after it, flags included. `--` is needed only before\n\
        an argument that is one of run's own, --bin, -h, or --help. The\n\
        binaries are the workspace members' bin targets, as `cargo metadata`\n\
        reports them, xtask's own left out, and with exactly one there is\n\
        nothing to ask. With none or several the run is refused, the\n\
        binaries listed, and --bin names the one to run.")]
    Run {
        /// The binary to run, when the workspace has more than one
        #[arg(long, value_name = "NAME", add = ArgValueCandidates::new(bin_candidates))]
        bin: Option<String>,
        /// The binary's own arguments, `--` first only when one is run's own
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        args: Vec<String>,
    },

    /// Print a bash function that completes `cargo xtask` and its aliases
    #[command(long_about = "\
        Print a bash function that completes `cargo xtask`, and each NAME, a\n\
        shell alias for it such as `cx`. It holds no task names: on each tab\n\
        it asks the xtask of the repo the shell is in, so one copy in a\n\
        shell's startup file serves every repo whose xtask answers this way.\n\n\
        `cargo xtask <tab>` reaches it through cargo's own bash completion,\n\
        which hands a subcommand it does not know to the completion\n\
        registered for `cargo-<name>`, and needs bash-completion 2.12 or\n\
        later. A NAME is completed directly.\n\n\
        Save it once and source that file, the function being the same in\n\
        every repo:\n\
        \x20 cargo xtask completions cx > ~/.config/cargo-xtask-completion.bash")]
    Completions {
        /// A shell alias for `cargo xtask`, completed as it is
        #[arg(value_name = "NAME")]
        names: Vec<String>,
    },
}

/// What `bump`'s argument completes to, beside a version as written.
fn bump_candidates() -> Vec<CompletionCandidate> {
    vec![
        CompletionCandidate::new("next")
            .help(Some("one more than the suffix's last number".into())),
        CompletionCandidate::new("bare").help(Some("the version without its suffix".into())),
    ]
}

/// What `run --bin` completes to: the workspace's binaries, this
/// package's own left out, read from `cargo metadata` on each tab. A
/// workspace that cannot be read completes nothing.
fn bin_candidates() -> Vec<CompletionCandidate> {
    let binaries = workspace_root()
        .and_then(|root| run::metadata(&root))
        .map(|m| run::binaries(&m, env!("CARGO_PKG_NAME")));
    match binaries {
        Ok(binaries) => binaries.into_iter().map(CompletionCandidate::new).collect(),
        Err(_) => Vec::new(),
    }
}

/// Answer a completion request with nothing when it is for the binary's
/// own arguments, `run`'s having ended, before clap's engine, which would
/// offer `run`'s flags there, is asked.
///
/// The request is this binary run with `COMPLETE` set, the command
/// line's words after a `--`, and `_CLAP_COMPLETE_INDEX` the word being
/// completed.
fn complete_nothing_past_runs_own() {
    if std::env::var_os("COMPLETE").is_none() {
        return;
    }
    let Some(index) = std::env::var("_CLAP_COMPLETE_INDEX")
        .ok()
        .and_then(|i| i.parse::<usize>().ok())
    else {
        return;
    };
    let words: Vec<String> = std::env::args().skip_while(|a| a != "--").skip(1).collect();
    if run::binary_args_started(&words, index) {
        std::process::exit(0);
    }
}

/// Run a task, returning what it prints.
fn dispatch(task: &Task) -> Result<String, String> {
    match task {
        Task::Completions { names } => Ok(completions::script(names).trim_end().to_string()),
        Task::Version => version::version(&workspace_root()?),
        Task::Bump { to } => version::bump(&workspace_root()?, &Bump::from_word(to)),
        Task::Validate { fast } => {
            let label = if *fast { "fast" } else { "full" };
            validate::validate(&workspace_root()?, &validate::commands(*fast), label)
        }
        Task::Run { bin, args } => run::run(
            &workspace_root()?,
            bin.as_deref(),
            args,
            env!("CARGO_PKG_NAME"),
        ),
    }
}

fn main() -> ExitCode {
    // With `COMPLETE` set these answer a completion request and exit.
    complete_nothing_past_runs_own();
    CompleteEnv::with_factory(Cli::command).complete();
    match dispatch(&Cli::parse().task) {
        Ok(said) => {
            // `run` says nothing of its own, the binary's output being all.
            if !said.is_empty() {
                println!("{said}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words the completion engine offers for `args`, the last of
    /// them the word being completed.
    fn completes(args: &[&str]) -> Vec<String> {
        let mut cmd = Cli::command();
        let words: Vec<std::ffi::OsString> = args.iter().map(Into::into).collect();
        clap_complete::engine::complete(&mut cmd, words, args.len() - 1, None)
            .expect("completes")
            .iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect()
    }

    /// The tasks complete by name, whatever the command was typed as,
    /// and `bump` offers its two words.
    #[test]
    fn the_tasks_and_bumps_words_complete() {
        let tasks = completes(&["cx", ""]);
        for task in ["version", "bump", "validate", "run", "completions"] {
            assert!(tasks.iter().any(|t| t == task), "{task}: {tasks:?}");
        }
        assert_eq!(completes(&["cargo-xtask", "v"]), ["version", "validate"]);
        assert_eq!(completes(&["cargo-xtask", "ve"]), ["version"]);
        assert_eq!(completes(&["cx", "bump", ""])[..2], ["next", "bare"]);
        assert_eq!(completes(&["cx", "bump", "n"]), ["next"]);
        // `run --bin` offers this workspace's binary, xtask's left out.
        assert_eq!(completes(&["cx", "run", "--bin", ""]), ["io-uring-x1"]);
    }

    /// The arguments parse as the hand-written ones did, and `-h` is
    /// help and no longer a usage error.
    #[test]
    fn the_arguments_are_claps() {
        let parse = |args: &[&str]| Cli::try_parse_from(args).map(|c| c.task);
        assert!(matches!(parse(&["xtask", "version"]), Ok(Task::Version)));
        assert!(matches!(parse(&["xtask", "bump", "next"]), Ok(Task::Bump { to }) if to == "next"));
        let help = parse(&["xtask", "-h"]).unwrap_err();
        assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
        assert!(help.to_string().contains("cargo xtask"), "{help}");
        assert!(matches!(
            parse(&["xtask", "validate"]),
            Ok(Task::Validate { fast: false })
        ));
        assert!(matches!(
            parse(&["xtask", "validate", "--fast"]),
            Ok(Task::Validate { fast: true })
        ));
        assert!(matches!(
            parse(&["xtask", "run"]),
            Ok(Task::Run { bin: None, args }) if args.is_empty()
        ));
        assert!(matches!(
            parse(&["xtask", "run", "--bin", "b", "--", "-x", "y"]),
            Ok(Task::Run { bin: Some(b), args }) if b == "b" && args == ["-x", "y"]
        ));
        // As `cargo run` takes them: from the first argument not run's
        // own, flags included, `--` only before one of run's own.
        let args = |argv: &[&str]| match parse(argv) {
            Ok(Task::Run { bin, args }) => Ok((bin, args)),
            other => Err(format!("{other:?}")),
        };
        let some = |b: &str| Some(b.to_string());
        assert_eq!(
            args(&["xtask", "run", "foo", "-x"]),
            Ok((None, vec!["foo".into(), "-x".into()]))
        );
        assert_eq!(
            args(&["xtask", "run", "-x", "foo"]),
            Ok((None, vec!["-x".into(), "foo".into()]))
        );
        assert_eq!(
            args(&["xtask", "run", "foo", "--bin", "b"]),
            Ok((None, vec!["foo".into(), "--bin".into(), "b".into()]))
        );
        assert_eq!(
            args(&["xtask", "run", "--bin", "b", "foo"]),
            Ok((some("b"), vec!["foo".into()]))
        );
        assert_eq!(
            args(&["xtask", "run", "--", "--bin"]),
            Ok((None, vec!["--bin".into()]))
        );
        assert_eq!(
            args(&["xtask", "run", "--", "--help"]),
            Ok((None, vec!["--help".into()]))
        );
        assert!(args(&["xtask", "run", "--help"]).is_err(), "run's own help");
        assert!(parse(&["xtask", "bump"]).is_err(), "bump needs its word");
        assert!(parse(&["xtask"]).is_err(), "a task is required");
    }
}
