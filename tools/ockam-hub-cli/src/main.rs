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

fn main() {
    match Cli::from_args() {
        Cli::Auth { app } => {
            println!("{} authentication!", app);
        }
    }
}
