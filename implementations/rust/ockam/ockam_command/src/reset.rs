use crate::CommandGlobalOpts;
use clap::Args;
use std::io::{self, BufReader, Read, Write};

/// Full Ockam Reset
#[derive(Clone, Debug, Args)]
pub struct ResetCommand {
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl ResetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if self.yes || get_user_confirmation() {
            if let Err(e) = run_impl(opts) {
                eprintln!("{e}");
                std::process::exit(e.code());
            }
        }
    }
}

fn run_impl(opts: CommandGlobalOpts) -> crate::Result<()> {
    opts.state.delete(true)?;
    Ok(())
}

fn get_user_confirmation() -> bool {
    let prompt = "Please confirm the you really want a full reset (y/N) ";
    print!("{prompt}");
    if io::stdout().flush().is_err() {
        // If stdout wasn't flushed properly, fallback to println
        println!("{prompt}");
    }
    let stdin = BufReader::new(io::stdin());
    stdin
        .bytes()
        .next()
        .and_then(|c| c.ok())
        .map(|c| c as char)
        .map(|c| (c == 'y' || c == 'Y'))
        .unwrap_or(false)
}
