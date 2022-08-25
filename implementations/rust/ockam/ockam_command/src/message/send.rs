use anyhow::Context;
use clap::Args;

use ockam::TcpTransport;
use ockam_api::clean_multiaddr;
use ockam_api::nodes::service::message::SendMessage;
use ockam_core::api::{Method, Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::util::api::CloudOpts;
use crate::util::{embedded_node, get_final_element, node_rpc, stop_node, RpcBuilder};
use crate::{CommandGlobalOpts, HELP_TEMPLATE};

const EXAMPLES: &str = "\
EXAMPLES

    # Create two nodes
    $ ockam node create n1
    $ ockam node create n2

    # Send a message to the uppercase service on node 1
    $ ockam message send hello --to /node/n1/service/uppercase
    HELLO

    # Send a message to the uppercase service on node n1 from node n1
    $ ockam message send hello --from /node/n2 --to /node/n1/service/uppercase
    HELLO

    # Create a secure channel from node n1 to the api service on node n2
    # Send a message through this encrypted channel to the uppercase service
    $ ockam secure-channel create --from /node/n1 --to /node/n2/service/api \
        | ockam message send hello --from /node/n1 --to -/service/uppercase
    HELLO

LEARN MORE
";

#[derive(Clone, Debug, Args)]
#[clap(help_template = const_str::replace!(HELP_TEMPLATE, "LEARN MORE", EXAMPLES))]
pub struct SendCommand {
    /// The node to send messages from
    #[clap(short, long, value_name = "NODE")]
    from: Option<String>,

    /// The route to send the message to
    #[clap(short, long, value_name = "ROUTE")]
    pub to: MultiAddr,

    /// Override Default Timeout
    #[clap(long, value_name = "TIMEOUT")]
    pub timeout: Option<u64>,

    pub message: String,

    #[clap(flatten)]
    cloud_opts: CloudOpts,
}

impl SendCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: SendCommand) {
        if let Some(node) = &cmd.from {
            let node = get_final_element(node).to_string();
            node_rpc(send_message_via_connection_to_a_node, (opts, cmd, node))
        } else if let Err(e) = embedded_node(send_message_from_embedded_node, (opts, cmd)) {
            eprintln!("Ockam node failed: {:?}", e,);
        }
    }
}

async fn send_message_from_embedded_node(
    mut ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, SendCommand),
) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;
    let (to, _) = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
    if let Some(route) = ockam_api::multiaddr_to_route(&to) {
        ctx.send(route, cmd.message).await?;
        match cmd.timeout {
            Some(timeout) => {
                let message = ctx
                    .receive_duration_timeout::<String>(std::time::Duration::from_secs(timeout))
                    .await?;
                println!("{}", message);
            }
            None => {
                let message = ctx.receive::<String>().await?;
                println!("{}", message);
            }
        }
    }

    ctx.stop().await?;

    Ok(())
}

async fn send_message_via_connection_to_a_node(
    ctx: ockam::Context,
    (opts, cmd, api_node): (CommandGlobalOpts, SendCommand, String),
) -> crate::Result<()> {
    async fn go(
        ctx: &ockam::Context,
        opts: &CommandGlobalOpts,
        cmd: SendCommand,
        api_node: String,
    ) -> crate::Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let (to, meta) = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let projects_sc = crate::project::util::lookup_projects(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.addr,
            &api_node,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        let mut rpc = RpcBuilder::new(ctx, opts, &api_node).tcp(&tcp).build()?;
        rpc.request(req(&to, &cmd.message)).await?;
        let res = rpc.parse_response::<Vec<u8>>()?;
        println!(
            "{}",
            String::from_utf8(res).context("Received content is not a valid utf8 string")?
        );
        Ok(())
    }
    let result = go(&ctx, &opts, cmd, api_node).await;
    stop_node(ctx).await?;
    result
}

pub(crate) fn req<'a>(to: &'a MultiAddr, message: &'a str) -> RequestBuilder<'a, SendMessage<'a>> {
    Request::builder(Method::Post, "v0/message").body(SendMessage::new(to, message.as_bytes()))
}
