use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use ockam_node::Context;
use std::net::SocketAddr;

use crate::run::ConfigRunner;
use crate::tcp::inlet::create::default_from_addr;
use crate::util::node_rpc;
use crate::util::parsers::socket_addr_parser;
use crate::{docs, fmt_info, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/secure_relay_inlet/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/secure_relay_inlet/after_long_help.txt");

/// Create and setup a new relay node, idempotent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SecureRelayInlet {
    /// The name of the service
    #[arg(value_name = "SERVICE NAME")]
    pub service_name: String,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", default_value_t = default_from_addr(), value_parser = socket_addr_parser)]
    from: SocketAddr,

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

impl SecureRelayInlet {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), rpc, (opts, self))
    }
}

async fn rpc(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SecureRelayInlet),
) -> miette::Result<()> {
    cmd.create_config_and_start(ctx, opts).await
}

impl SecureRelayInlet {
    pub async fn create_config_and_start(
        self,
        ctx: Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let recipe = self.create_config_recipe();

        if self.dry_run {
            opts.terminal.write_line(recipe.as_str())?;
            return Ok(());
        }

        opts.terminal.write_line(fmt_info!(
            r#"Creating new inlet relay node using this configuration:
```
{}```
       You can copy and customize the above recipe and launch it with `ockam run`.
"#,
            recipe.as_str().dark_gray()
        ))?;

        ConfigRunner::run_config(&ctx, opts, &recipe).await
    }

    fn create_config_recipe(&self) -> String {
        let projects = if let Some(t) = self.enroll.enroll_ticket.as_ref() {
            formatdoc! {r#"
                projects:
                  enroll:
                    - ticket: {t}
            "#}
        } else {
            "".to_string()
        };

        let recipe: String = formatdoc! {
            r#"
            {projects}
            tcp-inlets:
              {service_name}:
                from: {from}
                to: {service_name}
            "#,
            from = self.from,
            service_name = self.service_name,
        };

        recipe
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::{ArgsToCommands, Config};
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::EnrollmentTicket;
    use std::str::FromStr;

    #[test]
    fn test_that_recipe_is_valid() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();

        let cmd = SecureRelayInlet {
            service_name: "service_name".to_string(),
            from: SocketAddr::from_str("127.0.0.1:8080").unwrap(),
            dry_run: false,
            enroll: Enroll {
                enroll_ticket: Some(enrollment_ticket_hex),
                okta: false,
            },
        };
        let config_recipe = cmd.create_config_recipe();
        let config = Config::parse(config_recipe.as_str()).unwrap();
        config.projects.into_commands().unwrap();
        config.tcp_inlets.into_commands().unwrap();
    }
}
