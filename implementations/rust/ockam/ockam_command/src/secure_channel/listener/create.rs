use clap::Args;
use colorful::Colorful;
use miette::{miette, WrapErr};

use crate::{docs, CommandGlobalOpts};
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelListenerRequest;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;
use ockam_core::Address;

use crate::node::util::initialize_default_node;
use crate::node::NodeOpts;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Address for this listener
    address: Address,

    /// Authorized Identifiers of secure channel initiators
    #[arg(short, long, value_name = "IDENTIFIERS")]
    authorized: Option<Vec<Identifier>>,

    /// Name of the Identity that the secure-channel listener will use
    /// If it is different from the default node identity
    #[arg(value_name = "IDENTITY_NAME", long)]
    identity: Option<String>,
}

impl CreateCommand {
    pub fn name(&self) -> String {
        "create secure channel listener".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let req = Request::post("/node/secure_channel_listener").body(
            CreateSecureChannelListenerRequest::new(
                &self.address,
                self.authorized.clone(),
                self.identity.clone(),
            ),
        );
        let result = node.tell(ctx, req).await;
        match result {
            Ok(_) => {
                let address = format!("/service/{}", self.address.address());
                opts.terminal
                    .stdout()
                    .plain(
                        fmt_ok!(
                            "Secure Channel Listener at {} created successfully\n",
                            address
                                .to_string()
                                .color(OckamColor::PrimaryResource.color())
                        ) + &fmt_log!(
                            "At node {}{}",
                            "/node/".color(OckamColor::PrimaryResource.color()),
                            node.node_name().color(OckamColor::PrimaryResource.color())
                        ),
                    )
                    .machine(address.to_string())
                    .json(serde_json::json!([{ "address": address }]))
                    .write_line()?;
                Ok(())
            }
            Err(e) => Err(miette!(
                "An error occurred while creating the secure channel listener"
            ))
            .context(e),
        }
    }
}
