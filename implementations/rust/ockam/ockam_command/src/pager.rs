use crate::util::exitcode;

use miette::IntoDiagnostic;
use ockam_core::env::get_env_with_default;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::process::Stdio;

pub(crate) fn render_help(help: clap::Error) {
    let pager = get_env_with_default("PAGER", "less".to_string()).expect("Invalid PAGER value");
    match which::which(pager) {
        Ok(pager_binary_path) => {
            paginate_with(pager_binary_path, help).expect("Failed to paginate help");
        }
        // The pager binary was not found, so we just print the help without pagination
        Err(_) => {
            help.exit();
        }
    }
}

fn paginate_with(pager_binary_path: PathBuf, help: clap::Error) -> miette::Result<()> {
    let pager = pager_binary_path.file_name().unwrap().to_string_lossy();
    let mut pager_cmd = process::Command::new(pager.as_ref());
    if pager.as_ref() == "less" {
        pager_cmd.env("LESS", "FRX");
        // - F: no pagination if the text fits entirely into the window
        // - R: allow ANSI escapes output formatting
        // - X: prevents clearing the screen on exit
        // - using env var in case a lesser `less` poses as `less`
    }
    let mut pager_process = pager_cmd.stdin(Stdio::piped()).spawn().into_diagnostic()?;
    {
        let mut pager_stdin = pager_process
            .stdin
            .take()
            .expect("Failed to get pager's stdin");
        pager_stdin
            .write_all(help.render().ansi().to_string().as_bytes())
            .into_diagnostic()?;
    }
    let _ = pager_process.wait();
    let code = if help.use_stderr() {
        exitcode::USAGE
    } else {
        exitcode::OK
    };
    process::exit(code);
}
