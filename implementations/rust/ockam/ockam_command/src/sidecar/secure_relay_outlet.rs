use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use ockam_node::Context;
use std::net::SocketAddr;

use crate::run::ConfigRunner;
use crate::util::node_rpc;
use crate::util::parsers::socket_addr_parser;
use crate::{docs, fmt_info, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/secure_relay_outlet/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/secure_relay_outlet/after_long_help.txt");

/// Create and setup a new relay node, idempotent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SecureRelayOutlet {
    /// The name of the service
    #[arg(value_name = "SERVICE NAME")]
    pub service_name: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
    to: SocketAddr,

    /// Just print the recipe and exit
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    enroll: Enroll,
}

#[derive(Clone, Debug, Args)]
#[group(required = true, multiple = false)]
struct Enroll {
    /// Enrollment ticket to use
    #[arg(
        long,
        value_name = "ENROLLMENT TICKET PATH",
        group = "authentication_method"
    )]
    pub enroll_ticket: Option<String>,

    /// If using Okta enrollment
    #[arg(long = "okta", group = "authentication_method")]
    pub okta: bool,
}

impl SecureRelayOutlet {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self))
    }
}

async fn rpc(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SecureRelayOutlet),
) -> miette::Result<()> {
    cmd.create_config_and_start(opts).await
}

impl SecureRelayOutlet {
    pub async fn create_config_and_start(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let stdout = opts.terminal.clone().stdout();

        let enrollment_ticket: String = if let Some(t) = self.enroll.enroll_ticket.as_ref() {
            format! {
                "enrollment-ticket: {t}",
            }
        } else {
            "okta: true".to_string()
        };

        let recipe: String = formatdoc! {
            r#"
            nodes:
              secure_relay_outlet:
                {enrollment_ticket}
                tcp-outlets:
                  {service_name}:
                    from: '/service/outlet_{service_name}'
                    to: {to}
                    access_control: '(= subject.component "{service_name}")'
                relays:
                  {service_name}:
                    at: /project/default
            "#,
            to = self.to.to_string(),
            service_name = self.service_name,
        };

        if self.dry_run {
            stdout.plain(recipe.as_str()).write_line()?;
            return Ok(());
        }

        stdout
            .plain(fmt_info!(
                r#"Creating new outlet relay node using this configuration:
```
{}```
       You can copy and customize the above recipe and launch it with `ockam run`.
"#,
                recipe.as_str().dark_gray()
            ))
            .write_line()?;

        ConfigRunner::go(opts, &recipe, true).await
    }
}
