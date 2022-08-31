use anyhow::Context;
use clap::Args;

use ockam::TcpTransport;
use ockam_api::clean_multiaddr;
use ockam_api::nodes::service::message::SendMessage;
use ockam_core::api::{Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::util::api::CloudOpts;
use crate::util::{embedded_node, get_final_element, node_rpc, RpcBuilder};
use crate::{help, message::HELP_DETAIL, CommandGlobalOpts};

/// Send messages
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
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
    pub fn run(self, options: CommandGlobalOpts) {
        if let Some(node) = &self.from {
            let node = get_final_element(node).to_string();
            node_rpc(send_message_via_connection_to_a_node, (options, self, node))
        } else if let Err(e) = embedded_node(send_message_from_embedded_node, (options, self)) {
            eprintln!("Ockam node failed: {:?}", e,);
        }
    }
}

async fn send_message_from_embedded_node(
    mut ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, SendCommand),
) -> crate::Result<()> {
    let (to, _) = clean_multiaddr(&cmd.to, &opts.config.get_lookup())
        .context("Argument '--to' is invalid")?;
    let route = ockam_api::multiaddr_to_route(&to).context("Argument '--to' is invalid")?;
    let _tcp = TcpTransport::create(&ctx).await?;
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
        let (to, meta) = clean_multiaddr(&cmd.to, &opts.config.get_lookup())
            .context("Argument '--to' is invalid")?;
        let tcp = TcpTransport::create(ctx).await?;
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.route_to_controller,
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
    go(&ctx, &opts, cmd, api_node).await
}

pub(crate) fn req<'a>(to: &'a MultiAddr, message: &'a str) -> RequestBuilder<'a, SendMessage<'a>> {
    Request::post("v0/message").body(SendMessage::new(to, message.as_bytes()))
}
