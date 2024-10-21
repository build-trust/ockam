use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use ockam::transport::HostnamePort;
use ockam_api::fmt_info;

use crate::{docs, CommandGlobalOpts};
use ockam_node::Context;

use crate::run::Config;
use crate::tcp::inlet::create::default_from_addr;
use crate::util::async_cmd;
use crate::util::parsers::hostname_parser;

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
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", default_value_t = default_from_addr(), value_parser = hostname_parser)]
    from: HostnamePort,

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show relay inlet".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.create_config_and_start(ctx, opts).await
    }

    pub async fn create_config_and_start(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let mut recipe = self.create_config_recipe();

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

        Config::parse_and_run(ctx, opts, &mut recipe).await
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
    use ockam_api::cli_state::ExportedEnrollmentTicket;

    #[test]
    fn test_that_recipe_is_valid() {
        let enrollment_ticket = ExportedEnrollmentTicket::new_test();
        let enrollment_ticket_encoded = enrollment_ticket.to_string();

        let cmd = SecureRelayInlet {
            service_name: "service_name".to_string(),
            from: HostnamePort::new("127.0.0.1", 8080),
            dry_run: false,
            enroll: Enroll {
                enroll_ticket: Some(enrollment_ticket_encoded),
                okta: false,
            },
        };
        let mut config_recipe = cmd.create_config_recipe();
        let config = Config::parse(&mut config_recipe).unwrap();
        config.project_enroll.into_parsed_commands(None).unwrap();
        config.tcp_inlets.into_parsed_commands(None).unwrap();
    }
}
