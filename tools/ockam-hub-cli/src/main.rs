mod auth;

use auth::github;
use std::process;
use structopt::StructOpt;

/// This tool is designed to help you to configure the Ockam Hub.
#[derive(StructOpt, Debug)]
enum Cli {
    Auth {
        /// The app with you want to authenticate
        #[structopt(default_value = "github")]
        app: String,
    },
}

fn auth_with(app: String) -> Result<(), process::ExitStatus> {
    match &app[..] {
        "github" => {
            github::authenticate();
            Ok(())
        }
        _ => {
            println!("No app `{}` found", app);
            process::exit(1)
        }
    }
}

fn main() -> Result<(), process::ExitStatus> {
    match Cli::from_args() {
        Cli::Auth { app } => auth_with(app),
    }
}
