use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use ockam_api::cloud::enroll::enrollment_token::EnrollmentToken;
use ockam_api::{Response, Status};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::{CommandGlobalOpts, MessageFormat};

#[derive(Clone, Debug, Args)]
pub struct GenerateEnrollmentTokenCommand {
    /// Ockam's cloud address
    #[clap(display_order = 1000)]
    address: MultiAddr,

    /// Attributes (use '=' to separate key from value)
    #[clap(value_delimiter('='), last = true, required = true)]
    pub attrs: Vec<String>,

    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl GenerateEnrollmentTokenCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: GenerateEnrollmentTokenCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, (opts, cmd), generate);
    }
}

async fn generate(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, GenerateEnrollmentTokenCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append("_internal.nodeman").into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::enroll::token_generate(&cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<EnrollmentToken>()?;
            let output = match opts.global_args.message_format {
                MessageFormat::Plain => format!("Token generated successfully: {:?}", body.token),
                MessageFormat::Json => serde_json::to_string(&body)?,
            };
            Ok(output)
        }
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => eprintln!("{err}"),
    };

    stop_node(ctx).await
}
