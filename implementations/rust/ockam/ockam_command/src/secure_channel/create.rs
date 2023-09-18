use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use serde_json::json;
use tokio::{sync::Mutex, try_join};

use ockam::{identity::Identifier, route, Context};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::route_to_multiaddr;
use ockam_multiaddr::MultiAddr;

use crate::docs;
use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::LocalNode;
use crate::project::util::{
    clean_projects_multiaddr, get_projects_secure_channels_from_config_lookup,
};
use crate::util::api::CloudOpts;
use crate::util::clean_nodes_multiaddr;
use crate::{
    error::Error,
    fmt_log, fmt_ok,
    terminal::OckamColor,
    util::{exitcode, node_rpc},
    CommandGlobalOpts,
};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    /// Node from which to initiate the secure channel
    #[arg(value_name = "NODE", long, display_order = 800)]
    pub from: String,

    /// Route to a secure channel listener
    #[arg(value_name = "ROUTE", long, display_order = 800)]
    pub to: MultiAddr,

    /// Identifiers authorized to be presented by the listener
    #[arg(value_name = "IDENTIFIER", long, short, display_order = 801)]
    pub authorized: Option<Vec<Identifier>>,

    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,

    /// Name of a stored Credential to use within this Secure Channel
    #[arg(short, long)]
    pub credential: Option<String>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(rpc, (opts, self));
    }

    // Read the `to` argument and return a MultiAddr
    // or exit with and error if `to` can't be parsed.
    async fn parse_to_route(
        &self,
        opts: &CommandGlobalOpts,
        ctx: &Context,
        node: &LocalNode,
    ) -> miette::Result<MultiAddr> {
        let (to, meta) = clean_nodes_multiaddr(&self.to, &opts.state)
            .into_diagnostic()
            .wrap_err(format!("Could not convert {} into route", &self.to))?;

        let identity_name = get_identity_name(&opts.state, &self.cloud_opts.identity);

        let projects_sc = get_projects_secure_channels_from_config_lookup(
            opts,
            ctx,
            node,
            &meta,
            Some(identity_name),
        )
        .await?;
        clean_projects_multiaddr(to, projects_sc)
            .into_diagnostic()
            .wrap_err("Could not parse projects from route")
    }

    // Read the `from` argument and return node name
    fn parse_from_node(&self) -> String {
        extract_address_value(&self.from).unwrap_or_else(|_| "".to_string())
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    opts.terminal
        .write_line(&fmt_log!("Creating Secure Channel...\n"))?;

    let from = &cmd.parse_from_node();
    let node = LocalNode::make(&ctx, &opts, None).await?;
    let to = cmd.parse_to_route(&opts, &ctx, &node).await?;

    let authorized_identifiers = cmd.authorized.clone();

    // Delegate the request to create a secure channel to the from node.
    let is_finished: Mutex<bool> = Mutex::new(false);

    let create_secure_channel = async {
        let identity = get_identity_name(&opts.state, &cmd.cloud_opts.identity);

        let sc = node
            .node_manager
            .create_monitored_secure_channel(
                &ctx,
                to,
                authorized_identifiers,
                CredentialExchangeMode::Mutual,
                None,
                Some(identity),
                cmd.credential.clone(),
            )
            .await?;
        *is_finished.lock().await = true;
        Ok(sc)
    };

    let output_messages = vec!["Creating Secure Channel...".to_string()];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (secure_channel, _) = try_join!(create_secure_channel, progress_output)?;

    let route = &route![secure_channel.encryptor_address().to_string()];
    let multi_addr = route_to_multiaddr(route).ok_or_else(|| {
        Error::new(
            exitcode::PROTOCOL,
            miette!("Failed to convert route {route} to multi-address"),
        )
    })?;

    let from = format!("/node/{}", from);
    opts.terminal
        .stdout()
        .plain(
            fmt_ok!(
                "Secure Channel at {} created successfully\n",
                multi_addr
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!(
                "From {} to {}",
                from.color(OckamColor::PrimaryResource.color()),
                cmd.to
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
        )
        .machine(multi_addr.to_string())
        .json(json!([{ "address": multi_addr.to_string() }]))
        .write_line()?;

    Ok(())
}
