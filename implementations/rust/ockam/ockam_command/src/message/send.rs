use anyhow::Context as _;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::clean_multiaddr;
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::nodes::service::message::SendMessage;
use ockam_core::api::{Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::node::util::{delete_embedded_node, start_embedded_node_with_vault_and_identity};
use crate::util::api::{CloudOpts, ProjectOpts};
use crate::util::{extract_address_value, node_rpc, RpcBuilder};
use crate::Result;
use crate::{help, message::HELP_DETAIL, CommandGlobalOpts};

/// Send messages
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct SendCommand {
    /// The node to send messages from
    #[arg(short, long, value_name = "NODE")]
    from: Option<String>,

    /// The route to send the message to
    #[arg(short, long, value_name = "ROUTE")]
    pub to: MultiAddr,

    /// Override Default Timeout
    #[arg(long, value_name = "TIMEOUT")]
    pub timeout: Option<u64>,

    pub message: String,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    project_opts: ProjectOpts,
}

impl SendCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self))
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, SendCommand)) -> Result<()> {
    async fn go(ctx: &mut Context, opts: &CommandGlobalOpts, cmd: SendCommand) -> Result<()> {
        // Process `--to` Multiaddr
        let (to, meta) =
            clean_multiaddr(&cmd.to, &opts.state).context("Argument '--to' is invalid")?;

        // Setup environment depending on whether we are sending the message from an embedded node or a background node
        let (api_node, tcp) = if let Some(node) = &cmd.from {
            let api_node = extract_address_value(node)?;
            let tcp = TcpTransport::create(ctx).await?;
            (api_node, Some(tcp))
        } else {
            let api_node = start_embedded_node_with_vault_and_identity(
                ctx,
                opts,
                None,
                cmd.cloud_opts.identity.as_ref(),
                Some(&cmd.project_opts),
            )
            .await?;
            (api_node, None)
        };

        // Replace `/project/<name>` occurrences with their respective secure channel addresses
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &meta,
            &cmd.cloud_opts.route(),
            &api_node,
            tcp.as_ref(),
            CredentialExchangeMode::Oneway,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        // Send request
        let mut rpc = RpcBuilder::new(ctx, opts, &api_node)
            .tcp(tcp.as_ref())?
            .build();
        rpc.request(req(&to, &cmd.message)).await?;
        let res = rpc.parse_response::<Vec<u8>>()?;
        println!(
            "{}",
            String::from_utf8(res).context("Received content is not a valid utf8 string")?
        );

        // only delete node in case 'from' is empty and embedded node was started before
        if cmd.from.is_none() {
            delete_embedded_node(opts, rpc.node_name()).await;
        }

        Ok(())
    }
    go(&mut ctx, &opts, cmd).await
}

pub(crate) fn req<'a>(to: &'a MultiAddr, message: &'a str) -> RequestBuilder<'a, SendMessage<'a>> {
    Request::post("v0/message").body(SendMessage::new(to, message.as_bytes()))
}
