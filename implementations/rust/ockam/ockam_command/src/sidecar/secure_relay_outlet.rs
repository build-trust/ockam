use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use ockam::transport::HostnamePort;
use ockam_api::fmt_info;

use crate::{docs, CommandGlobalOpts};
use ockam_node::Context;

use crate::run::Config;
use crate::util::async_cmd;
use crate::util::parsers::hostname_parser;

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
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = hostname_parser)]
    to: HostnamePort,

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show relay outlet".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.create_config_and_start(ctx, opts).await
    }

    pub async fn create_config_and_start(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let recipe: String = self.create_config_recipe();

        if self.dry_run {
            opts.terminal.write_line(recipe.as_str())?;
            return Ok(());
        }

        opts.terminal.write_line(fmt_info!(
            r#"Creating new outlet relay node using this configuration:
```
{}```
       You can copy and customize the above recipe and launch it with `ockam run`.
"#,
            recipe.as_str().dark_gray()
        ))?;

        Config::parse_and_run(ctx, opts, recipe).await
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

        let recipe: String = formatdoc! {r#"
            {projects}
            policies:
              - resource: 'tcp-outlet'
                expression: '(= subject.component "{service_name}")'
            tcp-outlets:
              {service_name}:
                to: {to}
            relays:
              - {service_name}
            "#,
            to = self.to,
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

        let cmd = SecureRelayOutlet {
            service_name: "service_name".to_string(),
            to: HostnamePort::new("127.0.0.1", 8080),
            dry_run: false,
            enroll: Enroll {
                enroll_ticket: Some(enrollment_ticket_encoded),
                okta: false,
            },
        };
        let config_recipe = cmd.create_config_recipe();
        let config = Config::parse(config_recipe).unwrap();
        config.project_enroll.into_parsed_commands(None).unwrap();
        config.policies.into_parsed_commands().unwrap();
        config.tcp_outlets.into_parsed_commands(None).unwrap();
        config.relays.into_parsed_commands(None).unwrap();
    }
}
