use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::{Context, Route};
use ockam_api::{nodes::NODEMAN_ADDR, Response, Status};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Transport ID
    pub id: String,

    /// Force this operation: delete the API transport if requested
    #[clap(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(opts: CommandGlobalOpts, command: DeleteCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, command, delete_transport);
    }
}

pub async fn delete_transport(
    ctx: Context,
    cmd: DeleteCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::delete_transport(&cmd)?,
        )
        .await
        .unwrap();

    let r: Response = api::parse_response(&resp)?;

    match r.status() {
        Some(Status::Ok) => eprintln!("Transport '{}' successfully deleted!", cmd.id),
        _ => {
            eprintln!("Failed to delete transport");
            if !cmd.force {
                eprintln!("You may have to provide --force to delete the API transport");
            }
        }
    }

    stop_node(ctx).await
}
