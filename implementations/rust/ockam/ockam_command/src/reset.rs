use crate::node::util::delete_all_nodes;
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
    pub fn run(self, options: CommandGlobalOpts) {
        let _ = run_impl(options, self);
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ResetCommand) -> crate::Result<()> {
    if cmd.yes || get_user_confirmation() {
        if let Err(e) = delete_all_nodes(opts, true) {
            eprintln!("{}", e);
            std::process::exit(crate::util::exitcode::IOERR);
        }
    }
    Ok(())
}

fn get_user_confirmation() -> bool {
    let prompt = "Please confirm the you really want a full reset (y/N) ";
    print!("{}", prompt);
    if io::stdout().flush().is_err() {
        // If stdout wasn't flushed properly, fallback to println
        println!("{}", prompt);
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
