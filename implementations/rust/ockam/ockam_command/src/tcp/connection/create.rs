use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;
use serde_json::json;

use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::output::OutputFormat;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct TcpConnectionNodeOpts {
    /// Node that will initiate the connection
    #[arg(global = true, short, long, value_name = "NODE", value_parser = extract_address_value)]
    pub from: Option<String>,
}

/// Create a TCP connection
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TcpConnectionNodeOpts,

    /// The address to connect to
    #[arg(id = "to", short, long, value_name = "ADDRESS")]
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create tcp connection".into()
    }

    #[allow(unused)]
    async fn print_output(
        &self,
        opts: &CommandGlobalOpts,
        response: &TransportStatus,
    ) -> miette::Result<()> {
        // if output format is json, write json to stdout.
        match opts.global_args.output_format {
            OutputFormat::Plain => {
                if !opts.terminal.is_tty() {
                    println!("{}", response.multiaddr().into_diagnostic()?);
                    return Ok(());
                }
                let from = opts
                    .state
                    .get_node_or_default(&self.node_opts.from)
                    .await?
                    .name();
                let to = response.socket_addr().into_diagnostic()?;
                if opts.global_args.no_color {
                    println!("\n  TCP Connection:");
                    println!("    From: /node/{from}");
                    println!("    To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port());
                    println!("    Address: {}", response.multiaddr().into_diagnostic()?);
                } else {
                    println!("\n  TCP Connection:");
                    println!("{}", format!("    From: /node/{from}").light_magenta());
                    println!(
                        "{}",
                        format!("    To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port())
                            .light_magenta()
                    );
                    println!(
                        "{}",
                        format!("    Address: {}", response.multiaddr().into_diagnostic()?)
                            .light_magenta()
                    );
                }
            }
            OutputFormat::Json => {
                let json = json!([{"route": response.multiaddr().into_diagnostic()? }]);
                println!("{json}");
            }
        }
        Ok(())
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.from).await?;
        let payload = models::transport::CreateTcpConnection::new(self.address.clone());
        let request = Request::post("/node/tcp/connection").body(payload);

        let transport_status: TransportStatus = node.ask(ctx, request).await?;
        let from = opts
            .state
            .get_node_or_default(&self.node_opts.from)
            .await?
            .name();
        let to = transport_status.socket_addr().into_diagnostic()?;
        let plain = formatdoc! {r#"
        TCP Connection:
            From: /node/{from}
            To: {to} (/ip4/{}/tcp/{})
            Address: {}
    "#, to.ip(), to.port(), transport_status.multiaddr().into_diagnostic()?};
        let json = json!([{"route": transport_status.multiaddr().into_diagnostic()? }]);
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
