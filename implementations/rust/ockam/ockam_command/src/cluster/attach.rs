use crate::cluster::common_args::{
    EnrollmentTicketConfigArg, HttpApiArgs, ZoneConfigArg, ZoneNameOrConfigArg,
};
use crate::cluster::ctrlc::ClusterCtrlcHandler;
use crate::cluster::repl::ReplCommand;
use crate::cluster::zone_config::{Outlet, ZoneConfig};
use crate::entry_point::RUNTIME;
use crate::util::port_is_free_guard;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::address::get_free_address;
use ockam_api::colors::color_primary;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_api::{fmt_log, fmt_ok, fmt_separator};
use ockam_node::{Context, Executor, NodeBuilder};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;

const LONG_ABOUT: &str = include_str!("./static/attach/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/attach/after_long_help.txt");

/// Open a repl to an Outlet of an Ockam AI Zone
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
    pub http_api: HttpApiArgs,
}

#[async_trait]
impl Command for AttachCommand {
    const NAME: &'static str = "cluster attach";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        self.run_impl(opts, None).await
    }
}

impl AttachCommand {
    pub(crate) async fn run_impl(
        self,
        opts: CommandGlobalOpts,
        restart_tx: Option<tokio::sync::broadcast::Sender<String>>,
    ) -> Result<()> {
        let zone_config = self.zone.zone_config()?;
        let (executors, repl_address) = self.cluster_inlets(&opts, &zone_config).await?;
        // let (repl_data, rest_inlet_handles) = cmd.dummy_inlet(&opts).await?;
        opts.terminal.write_line(fmt_separator!())?;
        if let Some(address) = repl_address {
            let repl_command = ReplCommand {
                to: address.clone(),
            };
            repl_command.open_repl(&opts, address, restart_tx).await?;
        } else {
            // No repl outlet. Create a ctrlc handler and wait for it to be triggered before exiting the command
            let mut rx = ClusterCtrlcHandler::rx();
            let portals_str = if executors.len() > 1 {
                "all portals"
            } else {
                "the portal"
            };
            opts.terminal
                .write_line(fmt_log!("Press Ctrl+C to stop {portals_str} and exit"))?;
            let _ = rx.recv().await;
        }
        Ok(())
    }

    async fn cluster_inlets(
        &self,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<(Vec<Executor>, Option<SchemeHostnamePort>)> {
        let mut executors = Vec::new();
        let main_pod = zone_config.get_main_pod()?;
        let main_pod_outlets = main_pod.get_outlets();
        let create_inlet = |outlet: Outlet| async move {
            let from = Self::get_address_for_inlet(&outlet)?;
            let to = outlet.name.as_ref().unwrap_or(&main_pod.name);
            let pod_name = outlet.pod_name.as_ref().unwrap_or(&main_pod.name);
            let executor = self
                .cluster_inlet(opts, &zone_config.name, pod_name, from.clone(), to)
                .await?;
            Ok::<(Executor, Option<SchemeHostnamePort>), miette::Error>((executor, Some(from)))
        };
        let repl_address = match main_pod_outlets.repl {
            None => None,
            Some(repl_outlet) => {
                let (executor, repl_address) = create_inlet(repl_outlet).await?;
                executors.push(executor);
                repl_address
            }
        };
        let rest: Vec<Outlet> = main_pod_outlets
            .rest
            .into_iter()
            .chain([main_pod_outlets.http, main_pod_outlets.logs])
            .collect();
        for outlet in rest {
            let (executor, _) = create_inlet(outlet).await?;
            executors.push(executor);
        }
        Ok((executors, repl_address))
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
    async fn cluster_inlet(
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

        use crate::cluster::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone: ZoneNameOrConfigArg::from_zone_name(zone_name.to_string()),
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
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

#[allow(dead_code)]
pub(super) mod dummy_server {
    use super::*;

    impl AttachCommand {
        /// Start a TCP server that echoes back any input
        pub(super) async fn dummy_inlet(
            &self,
            opts: &CommandGlobalOpts,
            inlet_address: SchemeHostnamePort,
        ) -> miette::Result<(Option<JoinHandle<Result<()>>>, Vec<JoinHandle<Result<()>>>)> {
            use tokio::net::TcpListener;

            let addr = inlet_address.hostname_port().to_string();
            let inlet_handle = tokio::spawn(async move {
                let listener = TcpListener::bind(&addr)
                    .await
                    .into_diagnostic()
                    .wrap_err(format!("Failed to bind to {}", addr))?;
                loop {
                    let (socket, _) = listener
                        .accept()
                        .await
                        .into_diagnostic()
                        .wrap_err("Failed to accept connection")?;
                    tokio::spawn(async move {
                        if let Err(err) = Self::handle_connection(socket).await {
                            eprintln!("Connection error: {:?}", err);
                        }
                    });
                }
            });

            opts.terminal.write_line(fmt_ok!(
                "Dummy echo inlet started on {}",
                color_primary(inlet_address.to_string())
            ))?;

            // Ok((Some(inlet_handle), vec![]))
            Ok((None, vec![inlet_handle]))
        }

        async fn handle_connection(socket: tokio::net::TcpStream) -> miette::Result<()> {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);

            // Initial message
            let welcome =
                "Welcome to the dummy echo inlet. Type anything and it will be echoed back.\n";
            Self::send_formatted_response(&mut writer, welcome).await?;

            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        return Ok(());
                    }
                    Ok(_n) => {}
                    Err(e) => {
                        return Err(miette!("Failed to read from socket: {}", e));
                    }
                };

                if buffer.trim().is_empty() {
                    continue;
                }

                // Echo data back
                let message = format!("Echo: {}", buffer);
                Self::send_formatted_response(&mut writer, &message).await?;
            }
        }

        async fn send_formatted_response(
            writer: &mut tokio::net::tcp::OwnedWriteHalf,
            message: &str,
        ) -> miette::Result<()> {
            // First line contains just the length
            let header = format!("{}\n", message.len());
            writer
                .write_all(header.as_bytes())
                .await
                .into_diagnostic()
                .wrap_err("Failed to write header to socket")?;
            writer
                .write_all(message.as_bytes())
                .await
                .into_diagnostic()
                .wrap_err("Failed to write message to socket")?;
            writer
                .flush()
                .await
                .into_diagnostic()
                .wrap_err("Failed to flush message")?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::zone_config::Outlet;
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
