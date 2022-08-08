use crate::{
    node::NodeOpts,
    util::{api, connect_to, stop_node},
    CommandGlobalOpts,
};
use clap::Args;
use ockam::{Context, Route, TCP};
use ockam_api::{
    config::snippet::{ComposableSnippet, Operation, Protocol, RemoteMode},
    nodes::{models::transport::TransportStatus, NODEMANAGER_ADDR},
    route_to_multiaddr, Status,
};

#[derive(Args, Clone, Debug)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    pub bind: String,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let mode = RemoteMode::Listener;
        let tcp = true;
        let address = cc.bind.clone();

        Self {
            id: format!(
                "_transport_{}_{}_{}",
                mode,
                if tcp { "tcp" } else { "unknown" },
                address
            ),
            op: Operation::Transport {
                protocol: Protocol::Tcp,
                address,
                mode,
            },
            params: vec![],
        }
    }
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command.clone(), create_listener);

        let composite = (&command).into();
        let node = command.node_opts.api_node;

        let startup_config = match cfg.get_launch_config(&node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(-1);
            }
        };
        startup_config.add_composite(composite);
        if let Err(e) = startup_config.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(-1);
        }
    }
}

pub async fn create_listener(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = match ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_tcp_listener(&cmd)?,
        )
        .await
    {
        Ok(sr_msg) => sr_msg,
        Err(e) => {
            eprintln!("Wasn't able to send or receive `Message`: {}", e);
            std::process::exit(-1)
        }
    };

    let (response, TransportStatus { payload, .. }) = api::parse_transport_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            let r: Route = base_route
                .modify()
                .pop_back()
                .append_t(TCP, payload.to_string())
                .into();
            let multiaddr = match route_to_multiaddr(&r) {
                Some(addr) => addr,
                None => {
                    eprintln!("Couldn't convert given address into `MultiAddr`");
                    std::process::exit(-1)
                }
            };

            println!(
                "Tcp listener created! You can send messages to it via this route:\n`{}`",
                multiaddr
            )
        }
        _ => {
            eprintln!(
                "An error occurred while creating the tcp listener: {}",
                payload
            )
        }
    }
    stop_node(ctx).await
}
