use crate::cluster::common_args::HttpApiArgs;
use crate::entry_point::RUNTIME;
use crate::node_command::InMemoryNodeCommand;
use crate::util::port_is_free_guard;
use crate::zone::common_args::{
    EnrollmentTicketConfigArg, ZoneConfigArg, ZoneInletsArgs, ZoneNameOrConfigArg,
};
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::get_cluster_name::GetClusterName;
use crate::zone::repl::ReplCommand;
use crate::zone::zone_config::{Outlet, ZoneConfig};
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use ockam::transport::SchemeHostnamePort;
use ockam_api::address::get_free_address;
use ockam_api::colors::color_primary;
use ockam_api::{fmt_log, fmt_ok, fmt_separator};
use ockam_node::{Context, Executor, NodeBuilder};
use std::net::SocketAddr;
use std::str::FromStr;

const LONG_ABOUT: &str = include_str!("./static/attach/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/attach/after_long_help.txt");

/// Open the portals to an Ockam AI Zone
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct AttachCommand {
    #[command(flatten)]
    pub zone: ZoneConfigArg,

    #[command(flatten)]
    pub inlets: ZoneInletsArgs,

    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[async_trait]
impl Command for AttachCommand {
    const NAME: &'static str = "zone attach";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        self.run_impl(opts, ctx).await
    }
}

impl AttachCommand {
    pub async fn run_impl(self, opts: CommandGlobalOpts, ctx: &Context) -> Result<()> {
        let zone_config = self.zone.zone_config()?;
        let (executors, services_addresses) = self.zone_inlets(&opts, &zone_config).await?;

        opts.terminal.write_line(fmt_separator!())?;

        // Print the http server URL, if enabled
        if !self.inlets.no_http {
            let cluster = GetClusterName.execute(ctx, opts.state.clone()).await?;
            if let Some(http_url) = zone_config.get_http_url(&cluster) {
                let http_local_address = if let Some(http_address) = services_addresses.http {
                    let local = format!("http://localhost:{}\n", http_address.port());
                    &fmt_log!("{}", color_primary(local))
                } else {
                    ""
                };
                opts.terminal.write_line(
                    fmt_log!(
                        "The http server on the {} is available at:\n",
                        color_primary(&zone_config.get_main_pod()?.name),
                    ) + &fmt_log!("{}\n", color_primary(http_url))
                        + http_local_address,
                )?;
            }
        }

        if let Some(logs_address) = services_addresses.logs {
            let local_logs = format!("http://localhost:{}\n", logs_address.port());
            opts.terminal.write_line(
                fmt_log!("Logs for this zone are available at:\n")
                    + &fmt_log!("{}\n", color_primary(local_logs)),
            )?;
        }

        if let Some(to) = services_addresses.repl {
            let repl_command = ReplCommand { to };
            repl_command.open_repl(&opts).await
        } else {
            // No repl outlet. Wait for ctrlc to exit the command
            let portals_str = if executors.len() > 1 {
                "all portals"
            } else {
                "the portal"
            };
            opts.terminal
                .write_line(fmt_log!("Press Ctrl+C to stop {portals_str} and exit"))?;
            let _ = ZoneCtrlcHandler::wait_for_message().await;
            Ok(())
        }
    }

    async fn zone_inlets(
        &self,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<(Vec<Executor>, ServicesAddresses)> {
        let mut executors = Vec::new();
        let main_pod = zone_config.get_main_pod()?;
        let main_pod_outlets = main_pod.get_outlets();
        let create_inlet = |outlet: Outlet| async move {
            let from = Self::get_address_for_inlet(&outlet)?;
            let to = outlet.name.as_ref().unwrap_or(&main_pod.name);
            let pod_name = outlet.pod_name.as_ref().unwrap_or(&main_pod.name);
            let executor = self
                .zone_inlet(opts, &zone_config.name, pod_name, from.clone(), to)
                .await?;
            Ok::<(Executor, Option<SchemeHostnamePort>), miette::Error>((executor, Some(from)))
        };
        let mut rest = main_pod_outlets.rest;
        if !self.inlets.no_http {
            rest.push(main_pod_outlets.http.clone());
        }
        if !self.inlets.no_logs {
            rest.push(main_pod_outlets.logs.clone());
        }
        let mut http_address = None;
        let mut logs_address = None;
        for outlet in rest {
            let (executor, inlet_address) = create_inlet(outlet.clone()).await?;
            if outlet == main_pod_outlets.http {
                http_address = inlet_address
            } else if outlet == main_pod_outlets.logs {
                logs_address = inlet_address
            }
            executors.push(executor);
        }
        let repl_address = match main_pod_outlets.repl {
            None => None,
            Some(repl_outlet) => {
                let (executor, repl_address) = create_inlet(repl_outlet).await?;
                executors.push(executor);
                repl_address
            }
        };
        let services = ServicesAddresses {
            http: http_address,
            logs: logs_address,
            repl: repl_address,
        };
        Ok((executors, services))
    }

    fn get_address_for_inlet(outlet: &Outlet) -> miette::Result<SchemeHostnamePort> {
        let outlet_port = match outlet.get_port() {
            Some(port) => {
                let socket_addr =
                    SocketAddr::from_str(&format!("127.0.0.1:{port}")).into_diagnostic()?;
                if port_is_free_guard(&socket_addr).is_ok() {
                    port
                } else {
                    get_free_address()?.port()
                }
            }
            None => get_free_address()?.port(),
        };
        SchemeHostnamePort::from_str(&format!("localhost:{outlet_port}")).into_diagnostic()
    }

    #[allow(clippy::too_many_arguments)]
    async fn zone_inlet(
        &self,
        opts: &CommandGlobalOpts,
        zone_name: &str,
        pod_name: &str,
        from: SchemeHostnamePort,
        to: &str,
    ) -> miette::Result<Executor> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Opening a portal to the outlet {} on {} from {}...",
                color_primary(to),
                color_primary(pod_name),
                color_primary(from.to_string())
            ));
        }

        // Disable terminal output for the following commands
        let mut no_output_opts = opts.clone();
        no_output_opts.terminal = opts.terminal.disable();

        use crate::zone::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone: ZoneNameOrConfigArg::from_zone_name(zone_name.to_string()),
            http_api: self.http_api.clone(),
            pod: pod_name.to_string(),
            enrollment_ticket: EnrollmentTicketConfigArg {
                enrollment_ticket: None,
            },
            from: from.clone(),
            to: Some(to.to_string()),
            background: true,
            no_ctrlc_handler: true,
            ..Default::default()
        };
        let (ctx, executor) = {
            let rt = RUNTIME
                .get()
                .ok_or_else(|| miette!("Failed to get the runtime"))?
                .clone();
            NodeBuilder::new()
                .with_runtime(rt)
                .with_logging(false)
                .build()
        };
        inlet_command.run(&ctx, no_output_opts).await?;

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Opened a portal to the outlet {} on {} from {}",
            color_primary(to),
            color_primary(pod_name),
            color_primary(from.to_string())
        ))?;

        Ok(executor)
    }
}

/// This struct stores the inlet addresses created to access remote services
#[derive(Clone, Debug, PartialEq, Eq)]
struct ServicesAddresses {
    http: Option<SchemeHostnamePort>,
    logs: Option<SchemeHostnamePort>,
    repl: Option<SchemeHostnamePort>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::zone_config::Outlet;
    use std::net::TcpListener;
    use tokio::time::Duration;

    #[test]
    fn test_get_address_for_outlet_with_port() -> miette::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").into_diagnostic()?;
        let test_port = listener.local_addr().into_diagnostic()?.port();
        drop(listener);
        std::thread::sleep(Duration::from_secs(1));

        // Create an outlet with a specific port
        let outlet = Outlet {
            name: Some("test-outlet".to_string()),
            to: format!("localhost:{}", test_port),
            ..Default::default()
        };

        let address = AttachCommand::get_address_for_inlet(&outlet)?;
        assert_eq!(address.port(), test_port);

        Ok(())
    }

    #[test]
    fn test_get_address_for_outlet_with_occupied_port() -> miette::Result<()> {
        // Bind to a port and keep it occupied
        let listener = TcpListener::bind("127.0.0.1:0").into_diagnostic()?;
        let occupied_port = listener.local_addr().into_diagnostic()?.port();

        // Create an outlet with the occupied port
        let outlet = Outlet {
            name: Some("test-outlet".to_string()),
            to: format!("localhost:{}", occupied_port),
            ..Default::default()
        };

        let address = AttachCommand::get_address_for_inlet(&outlet)?;
        assert_ne!(address.port(), occupied_port);

        Ok(())
    }
}
