mod auth;

use auth::github;
use owo_colors::OwoColorize;
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

async fn auth_with(app: String) -> Result<(), process::ExitStatus> {
    match &app[..] {
        "github" => {
            if let Err(error) = github::authenticate().await {
                println!("Error authenticating github {:?}", error);
                process::exit(1)
            };
            Ok(())
        }
        _ => {
            println!("No app `{}` found", app);
            process::exit(1)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), process::ExitStatus> {
    match Cli::from_args() {
        Cli::Auth { app } => {
            println!(
                "{}{}\n",
                "
 .88888.           dP
d8'   `8b          88
88     88 .d8888b. 88  .dP  .d8888b. 88d8b.d8b.
88     88 88'  `\"\" 88888\"   88'  `88 88'`88'`88
Y8.   .8P 88.  ... 88  `8b. 88.  .88 88  88  88
 `8888P'  `88888P' dP   `YP `88888P8 dP  dP  dP
"
                .truecolor(82, 199, 234),
                "-----------------------------------------------".truecolor(236, 67, 45)
            );

            auth_with(app).await
        }
    }
}
