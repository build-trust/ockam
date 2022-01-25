use crate::command::channel::ChannelCommand;
use crate::command::channel_listen::ChannelListenCommand;
use crate::command::inlet::InletCommand;
use crate::command::outlet::OutletCommand;
use crate::AppError;
use clap::Parser;
use log::error;
use ockam::Context;

#[derive(Debug, Parser)]
#[clap(about, version, author)]
struct Args {
    #[clap(long, help = "Path to file containing a SSH private key")]
    ssh_private_key: Option<String>,

    #[clap(long, help = "Path to file containing a SSH public key")]
    ssh_public_key: Option<String>,

    #[clap(subcommand)]
    command: OckamCommand,
}

#[derive(Debug, clap::Subcommand)]
enum OckamCommand {
    ChannelListen {
        #[clap(short, long)]
        listen: String,

        #[clap(short, long, default_value = "secure_channel")]
        name: String,
    },
    Channel {
        #[clap(short, long)]
        channel: String,

        #[clap(short, long, default_value = "secure_channel")]
        name: String,

        #[clap(short, long)]
        message: String,
    },
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

pub struct AppConfig {}

impl AppConfig {
    pub async fn evaluate(ctx: &mut Context) -> Result<(), AppError> {
        let mut args: Args = Args::parse();
        let mut config = config::Config::default();
        let config = config
            .merge(config::Environment::with_prefix("OCKAM"))
            .unwrap();

        if args.ssh_private_key.is_none() {
            if let Ok(private_key) = config.get_str("ssh_private_key") {
                args.ssh_private_key = Some(private_key);
            }
        }

        if args.ssh_public_key.is_none() {
            if let Ok(public_key) = config.get_str("ssh_public_key") {
                args.ssh_public_key = Some(public_key);
            }
        }

        match &args.command {
            OckamCommand::ChannelListen { listen, name } => {
                if args.ssh_public_key.is_none() {
                    error!("Secure Channel Listener requires a public key.");
                    return Ok(());
                }
                ChannelListenCommand::run(ctx, args.ssh_public_key.unwrap(), listen, name).await
            }
            OckamCommand::Channel {
                channel,
                name,
                message,
            } => {
                if args.ssh_private_key.is_none() {
                    error!("Secure Channel requires a private key.");
                    return Ok(());
                }
                ChannelCommand::run(ctx, args.ssh_private_key.unwrap(), channel, name, message)
                    .await
            }
            OckamCommand::Outlet {
                listen,
                name,
                target,
            } => OutletCommand::run(ctx, listen, name, target).await,
            OckamCommand::Inlet {
                listen,
                outlet,
                name_outlet: outlet_name,
            } => InletCommand::run(ctx, listen, outlet, outlet_name).await,
        }
    }
}
