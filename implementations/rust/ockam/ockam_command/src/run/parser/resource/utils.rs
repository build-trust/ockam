use clap::Parser;
use miette::IntoDiagnostic;
use once_cell::sync::Lazy;
use std::process::Stdio;

use crate::{OckamCommand, OckamSubcommand};

static BINARY_PATH: Lazy<String> = Lazy::new(|| {
    std::env::args()
        .next()
        .expect("Failed to get the binary path")
});

pub fn binary_path() -> &'static str {
    &BINARY_PATH
}

/// Return a clap OckamSubcommand instance given the name of the
/// command and the list of arguments
pub fn parse_cmd_from_args(cmd: &str, args: &[String]) -> miette::Result<OckamSubcommand> {
    let args = [binary_path()]
        .into_iter()
        .chain(cmd.split(' '))
        .chain(args.iter().map(|s| s.as_str()))
        .collect::<Vec<_>>();
    Ok(OckamCommand::try_parse_from(args)
        .into_diagnostic()?
        .subcommand)
}

pub fn subprocess_stdio(quiet: bool) -> Stdio {
    if quiet {
        // If we're running in quiet mode, we don't need to propagate
        // the stdout/stderr to the child process
        Stdio::null()
    } else {
        // Otherwise, we need to inherit the stdout/stderr of the current process
        // to see the output written in the spawned process
        Stdio::inherit()
    }
}
