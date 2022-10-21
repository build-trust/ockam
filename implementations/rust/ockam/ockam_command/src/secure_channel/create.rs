use crate::{
    help,
    util::{api, exitcode, extract_address_value, node_rpc},
    CommandGlobalOpts, OutputFormat, Result,
};

use anyhow::Context as _;
use atty::Stream;
use clap::Args;
use colorful::Colorful;
use serde_json::json;

use crate::secure_channel::HELP_DETAIL;
use crate::util::api::CloudOpts;
use crate::util::RpcBuilder;
use ockam::{identity::IdentityIdentifier, route, Context, TcpTransport};
use ockam_api::config::lookup::ConfigLookup;
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::{
    clean_multiaddr, nodes::models::secure_channel::CreateSecureChannelResponse, route_to_multiaddr,
};
use ockam_multiaddr::MultiAddr;

/// Create Secure Channels
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node from which to initiate the secure channel (required)
    #[arg(value_name = "NODE", long, display_order = 800)]
    pub from: String,

    /// Route to a secure channel listener (required)
    #[arg(value_name = "ROUTE", long, display_order = 800)]
    pub to: MultiAddr,

    /// Identifiers authorized to be presented by the listener
    #[arg(value_name = "IDENTIFIER", long, short, display_order = 801)]
    pub authorized: Option<Vec<IdentityIdentifier>>,

    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    // Read the `to` argument and return a MultiAddr
    // or exit with and error if `to` can't be parsed.
    async fn parse_to_route(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        cloud_addr: &MultiAddr,
        api_node: &str,
        tcp: &TcpTransport,
    ) -> anyhow::Result<MultiAddr> {
        let config = &opts.config.lookup();
        let (to, meta) = clean_multiaddr(&self.to, config)
            .context(format!("Could not convert {} into route", &self.to))?;

        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &meta,
            cloud_addr,
            api_node,
            Some(tcp),
            CredentialExchangeMode::Oneway,
        )
        .await?;
        crate::project::util::clean_projects_multiaddr(to, projects_sc)
    }

    // Read the `from` argument and return node name
    fn parse_from_node(&self, _config: &ConfigLookup) -> String {
        extract_address_value(&self.from).unwrap_or_else(|_| "".to_string())
    }

    fn print_output(
        &self,
        parsed_from: &String,
        parsed_to: &MultiAddr,
        options: &CommandGlobalOpts,
        response: CreateSecureChannelResponse,
    ) {
        let route = &route![response.addr.to_string()];
        match route_to_multiaddr(route) {
            Some(multiaddr) => {
                // if stdout is not interactive/tty write the secure channel address to it
                // in case some other program is trying to read it as piped input
                if !atty::is(Stream::Stdout)
                // or if the `--pipe` flag is set
                || options.global_args.export.pipe()
                {
                    println!("{}", multiaddr)
                }

                // if output format is json, write json to stdout.
                if options.global_args.output_format == OutputFormat::Json {
                    let json = json!([{ "address": multiaddr.to_string() }]);
                    println!("{}", json);
                }

                // if stderr is interactive/tty and we haven't been asked to be quiet
                // and output format is plain then write a plain info to stderr.
                if atty::is(Stream::Stderr)
                    && !options.global_args.quiet
                    && options.global_args.output_format == OutputFormat::Plain
                {
                    if options.global_args.no_color {
                        eprintln!("\n  Created Secure Channel:");
                        eprintln!("  • From: /node/{}", parsed_from);
                        eprintln!("  •   To: {} ({})", &self.to, &parsed_to);
                        eprintln!("  •   At: {}", multiaddr);
                    } else {
                        eprintln!("\n  Created Secure Channel:");

                        // From:
                        eprint!("{}", "  • From: ".light_magenta());
                        eprintln!("{}", format!("/node/{}", parsed_from).light_yellow());

                        // To:
                        eprint!("{}", "  •   To: ".light_magenta());
                        let t = format!("{} ({})", &self.to, &parsed_to);
                        eprintln!("{}", t.light_yellow());

                        // At:
                        eprint!("{}", "  •   At: ".light_magenta());
                        eprintln!("{}", multiaddr.to_string().light_yellow());
                    }
                }
            }
            None => {
                // if stderr is interactive/tty and we haven't been asked to be quiet
                // and output format is plain then write a plain info to stderr.
                if atty::is(Stream::Stderr)
                    && !options.global_args.quiet
                    && options.global_args.output_format == OutputFormat::Plain
                {
                    eprintln!(
                        "Could not convert returned secure channel address {} into a multiaddr",
                        route
                    );
                }

                // return the exitcode::PROTOCOL since if things are going as expected
                // a route in the response should be convertible to multiaddr.
                std::process::exit(exitcode::PROTOCOL);
            }
        };
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;

    let config = &opts.config.lookup();
    let from = &cmd.parse_from_node(config);
    let to = &cmd
        .parse_to_route(&ctx, &opts, &cmd.cloud_opts.route(), from, &tcp)
        .await?;

    let authorized_identifiers = cmd.authorized.clone();

    // Delegate the request to create a secure channel to the from node.
    let mut rpc = RpcBuilder::new(&ctx, &opts, from).tcp(&tcp)?.build();
    let request =
        api::create_secure_channel(to, authorized_identifiers, CredentialExchangeMode::Mutual);

    rpc.request(request).await?;
    let response = rpc.parse_response::<CreateSecureChannelResponse>()?;

    cmd.print_output(from, to, &opts, response);

    Ok(())
}
