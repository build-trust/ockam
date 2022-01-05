use crate::command::inlet::InletCommand;
use crate::command::outlet::OutletCommand;
use crate::config::OckamCommand::{Inlet, Outlet};
use crate::AppError;
use clap::Parser;
use log::{debug, info};
use ockam::Context;

#[derive(Parser)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long, default_value = "ockam.toml")]
    config: String,

    #[clap(short, long, default_value = "ockam_secrets.toml")]
    secrets: String,

    #[clap(subcommand)]
    command: OckamCommand,
}

#[derive(clap::Subcommand)]
enum OckamCommand {
    Outlet {
        #[clap(short, long)]
        listen: String,

        #[clap(short, long, default_value = "outlet")]
        name: String,
        #[clap(short, long)]
        target: String,
    },

    Inlet {
        #[clap(short, long)]
        listen: String,

        #[clap(short, long)]
        outlet: String,

        #[clap(short, long, default_value = "outlet")]
        name_outlet: String,
    },
}

const OCKAM_ENV_PREFIX: &str = "OCKAM";

pub struct AppConfig {}

impl AppConfig {
    pub async fn evaluate(ctx: &Context) -> Result<(), AppError> {
        let mut config = config::Config::default();

        let args = Args::parse();

        if config.merge(config::File::with_name(&args.config)).is_ok() {
            info!("Loaded settings from {}.", args.config)
        } else {
            debug!("No config file present.")
        }

        if config.merge(config::File::with_name(&args.secrets)).is_ok() {
            info!("Loaded secrets from {}.", args.secrets)
        } else {
            debug!("No secrets file present.")
        }

        config
            .merge(config::Environment::with_prefix(OCKAM_ENV_PREFIX))
            .ok();

        match &args.command {
            Outlet {
                listen,
                name,
                target,
            } => OutletCommand::run(ctx, listen, name, target).await,
            Inlet {
                listen,
                outlet,
                name_outlet: outlet_name,
            } => InletCommand::run(ctx, listen, outlet, outlet_name).await,
        }
    }
}
