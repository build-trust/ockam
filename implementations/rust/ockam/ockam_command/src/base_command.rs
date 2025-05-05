use crate::branding::BrandingCompileEnvVars;
use crate::cluster::common_args::{HttpApiArgs, ZoneNameOrConfigArg};
use crate::cluster::zone_config::{Outlet, ZoneConfig};
use crate::util::port_is_free_guard;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use indicatif::ProgressBar;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::address::get_free_address;
use ockam_api::colors::color_primary;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_api::{fmt_log, fmt_ok, fmt_separator};
use ockam_core::env::get_env_ignore_error;
use ockam_core::TryClone;
use ockam_node::Context;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
    init_repository: String,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    async fn parse_args(mut self, _opts: &CommandGlobalOpts) -> Result<Self> {
        // load default values
        self.init_repository = "hello".to_string();

        // load env vars
        if let Some(v) = get_env_ignore_error("INIT_REPOSITORY") {
            self.init_repository = v;
        }

        Ok(self)
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;
        cmd.cluster_init(ctx, &opts).await?;
        let zone_config = cmd.cluster_create(ctx, &opts).await?;
        let (repl_data, rest_inlet_handles) = cmd.cluster_inlets(ctx, &opts, &zone_config).await?;
        // let (repl_data, rest_inlet_handles) = cmd.dummy_inlet(&opts).await?;
        opts.terminal.write_line(fmt_separator!())?;
        if let Some((address, handle)) = repl_data {
            cmd.open_repl(ctx, &opts, address, handle).await?;
        } else {
            // No REPL outlet. Create a ctrlc handler and wait for it to be triggered
            let (tx, mut rx) = tokio::sync::mpsc::channel(2);
            let mut processed = false;
            ctrlc::set_handler(move || {
                if !processed {
                    let _ = tx.blocking_send(());
                    processed = true
                }
            })
            .expect("Error setting exit signal handler");
            opts.terminal.write_line(fmt_log!(
                "Press Ctrl+C to stop the inlets and exit the command"
            ))?;
            rx.recv().await;
        }
        for handle in rest_inlet_handles {
            handle.abort();
        }
        Ok(())
    }

    async fn enroll(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let is_enrolled = opts
            .state
            .is_default_identity_enrolled()
            .await
            .is_ok_and(|v| v);
        if is_enrolled {
            return Ok(());
        }

        use crate::cluster::enroll::EnrollCommand;
        let enroll_command = EnrollCommand {
            disable_ctrlc_signal: true,
            ..Default::default()
        };
        enroll_command.run(ctx, opts.clone()).await?;
        opts.terminal.write_line("")?;

        Ok(())
    }

    async fn cluster_init(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let current_dir = std::env::current_dir()
            .into_diagnostic()
            .wrap_err("Failed to get current directory")?;
        if current_dir.read_dir().into_diagnostic()?.next().is_some() {
            return Ok(());
        }

        use crate::cluster::init::InitCommand;
        let init_command = InitCommand {
            repository: self.init_repository.clone(),
            target_path: None,
        };
        init_command.run(ctx, opts.clone()).await?;
        opts.terminal.write_line("")?;

        Ok(())
    }

    async fn cluster_create(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<ZoneConfig> {
        use crate::cluster::create::CreateCommand;
        let create_command = CreateCommand {
            use_public_ecr: true,
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let zone_config = create_command.run(ctx, opts.clone()).await?;
        Ok(zone_config)
    }

    async fn cluster_inlets(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<(
        Option<(SchemeHostnamePort, JoinHandle<Result<()>>)>,
        Vec<JoinHandle<Result<()>>>,
    )> {
        let mut handles = Vec::new();
        let main_pod = zone_config.get_main_pod()?;
        let main_pod_outlets = main_pod.get_outlets();
        let repl_data = match main_pod_outlets.repl {
            None => None,
            Some(repl_outlet) => {
                let from = Self::get_address_for_inlet(&repl_outlet)?;
                let to = repl_outlet.name.as_ref().unwrap_or(&main_pod.name);
                let handle = self
                    .cluster_inlet(
                        ctx,
                        opts,
                        &zone_config.name,
                        &main_pod.name,
                        from.clone(),
                        to,
                        false,
                    )
                    .await?;
                Some((from, handle))
            }
        };
        for outlet in main_pod_outlets.rest {
            let to = outlet.name.as_ref().unwrap_or(&main_pod.name);
            let from = Self::get_address_for_inlet(&outlet)?;
            let handle = self
                .cluster_inlet(ctx, opts, &zone_config.name, &main_pod.name, from, to, true)
                .await?;
            handles.push(handle);
        }
        Ok((repl_data, handles))
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
        ctx: &Context,
        opts: &CommandGlobalOpts,
        zone_name: &str,
        pod_name: &str,
        from: SchemeHostnamePort,
        to: &str,
        no_ctrlc_handler: bool,
    ) -> miette::Result<JoinHandle<Result<()>>> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Opening a Portal to the outlet {} from {}...",
                color_primary(to),
                color_primary(from.to_string())
            ));
        }

        // Disable terminal output for the following commands
        let mut no_output_opts = opts.clone();
        no_output_opts.terminal = opts.terminal.disable();

        use crate::cluster::ticket::TicketCommand;
        let ticket_command = TicketCommand {
            zone: ZoneNameOrConfigArg::from_zone_name(zone_name.to_string()),
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let ticket = ticket_command.run(ctx, no_output_opts.clone()).await?;

        use crate::cluster::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone: ZoneNameOrConfigArg::from_zone_name(zone_name.to_string()),
            pod: pod_name.to_string(),
            enrollment_ticket: Some(ticket),
            from: from.clone(),
            to: Some(to.to_string()),
            no_ctrlc_handler,
            ..Default::default()
        };
        let ctx = ctx.try_clone()?;
        let handle = tokio::spawn(async move { inlet_command.run(&ctx, no_output_opts).await });

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Portal connected to the outlet {} in {}",
            color_primary(to),
            color_primary(from.to_string())
        ))?;

        Ok(handle)
    }

    async fn open_repl(
        &self,
        _ctx: &Context,
        opts: &CommandGlobalOpts,
        inlet_address: SchemeHostnamePort,
        inlet_handle: JoinHandle<Result<()>>,
    ) -> miette::Result<()> {
        use tokio::net::TcpStream;
        use tokio::time::sleep;

        // Inlet monitor future
        let (inlet_tx, inlet_rx) = tokio::sync::oneshot::channel::<()>();
        let _inlet_monitor = tokio::spawn(async move {
            match inlet_handle.await {
                Ok(result) => result,
                Err(err) => {
                    let _ = inlet_tx.send(());
                    Err(miette!("{err}"))
                }
            }
        });

        // stdin quit handler
        let stdin = StdinHandler::start();
        let mut stdin_rx = stdin.tx.subscribe();
        let stdin_quit = async {
            loop {
                if let Ok(input) = stdin_rx.recv().await {
                    if is_quit_sequence(&input) {
                        break;
                    }
                }
            }
        };

        // Start the interactive REPL
        let opts = opts.clone();
        let mut stdin_rx = stdin.tx.subscribe();
        let repl_handle = tokio::spawn(async move {
            async fn connect_to_inlet(addr: &SchemeHostnamePort) -> miette::Result<TcpStream> {
                const MAX_RETRIES: u32 = 120; // Try for about 60 seconds (120 * 500ms)
                let duration = Duration::from_millis(500);
                let mut retries = 0;

                while retries < MAX_RETRIES {
                    match TcpStream::connect(addr.hostname_port().to_string()).await {
                        Ok(s) => return Ok(s),
                        Err(_) => {
                            retries += 1;
                            sleep(duration).await;
                        }
                    }
                }
                Err(miette!("Failed to connect to the TCP Inlet"))
            }

            let mut read_header_buffer = String::new();
            let mut read_body_buffer = Vec::new();
            let mut initial_message_spinner: Option<ProgressBar> = None;
            'repl: loop {
                // 1: Try to connect to the inlet
                let stream = match connect_to_inlet(&inlet_address).await {
                    Ok(s) => s,
                    Err(_e) => {
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                };

                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);

                // 2: Try to read initial message
                match Self::wait_until_server_is_ready(
                    &mut reader,
                    &mut read_header_buffer,
                    &mut read_body_buffer,
                )
                .await
                {
                    Ok(res) => {
                        // Successfully got initial message, continue to REPL
                        if let Some(spinner) = initial_message_spinner.take() {
                            spinner.finish_and_clear();
                        }
                        opts.terminal.write(res)?;
                    }
                    Err(_e) => {
                        if initial_message_spinner.is_none() {
                            initial_message_spinner = opts.terminal.spinner();
                        }
                        if let Some(spinner) = &initial_message_spinner {
                            spinner.set_message("Waiting for server to be ready...");
                        }
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                }

                // 3: Start the REPL loop for this connection
                'connection: loop {
                    // Get user input
                    let user_input = match stdin_rx.recv().await {
                        Ok(i) => i,
                        Err(_) => {
                            // stdin channel closed, exit REPL
                            break 'repl;
                        }
                    };

                    if user_input.trim().is_empty() {
                        continue 'connection;
                    }

                    if is_reconnect_sequence(&user_input) {
                        break 'connection;
                    }

                    if is_quit_sequence(&user_input) {
                        break 'repl;
                    }

                    // Send input to server
                    if let Err(e) = writer.write_all(user_input.as_bytes()).await {
                        debug!("failed to write to server: {:?}", e);
                        opts.terminal
                            .write_line("Failed to write to server. Reconnecting...")?;
                        break 'connection;
                    }
                    let _ = writer.flush().await;

                    // Wait for server response and print it
                    if let Err(e) = Self::process_server_response(
                        &opts,
                        &mut reader,
                        &mut read_header_buffer,
                        &mut read_body_buffer,
                    )
                    .await
                    {
                        opts.terminal.write_line(format!(
                            "Failed to read from server ({e}). Reconnecting...",
                        ))?;
                        break 'connection;
                    }
                }

                // If we break from the connection loop, we'll go back to connecting
                sleep(Duration::from_secs(1)).await;
            }

            Ok::<(), miette::Error>(())
        });

        // Wait for exit signals
        tokio::select! {
            _ = stdin_quit => {},
            _result = inlet_rx => {},
        }

        // Clean up tasks
        repl_handle.abort();

        Ok(())
    }
    async fn wait_until_server_is_ready(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
    ) -> std::result::Result<String, String> {
        match Self::read_server_response(
            reader,
            read_header_buffer,
            read_body_buffer,
            Duration::from_secs(5),
        )
        .await
        {
            ReadStatus::Success(res) => Ok(res),
            status => Err(status.to_string()),
        }
    }

    async fn process_server_response(
        opts: &CommandGlobalOpts,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
    ) -> std::result::Result<(), String> {
        let res = match Self::read_server_response(
            reader,
            read_header_buffer,
            read_body_buffer,
            Duration::from_secs(30),
        )
        .await
        {
            ReadStatus::Success(res) => Ok(res),
            ReadStatus::Timeout => {
                // Short timeout reached, notify but keep waiting
                let spinner = opts.terminal.spinner();
                if let Some(spinner) = &spinner {
                    spinner.set_message("Waiting for server response...");
                }

                // Continue with longer timeout
                let status = Self::read_server_response(
                    reader,
                    read_header_buffer,
                    read_body_buffer,
                    Duration::from_secs(5 * 60),
                )
                .await;

                if let Some(spinner) = &spinner {
                    spinner.finish_and_clear();
                }

                match status {
                    ReadStatus::Success(res) => Ok(res),
                    status => Err(status.to_string()),
                }
            }
            status => Err(status.to_string()),
        }?;
        opts.terminal.write(&res).map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn read_server_response(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
        timeout_duration: Duration,
    ) -> ReadStatus {
        // Use timeout only for the header read
        read_header_buffer.clear();
        match timeout(timeout_duration, reader.read_line(read_header_buffer)).await {
            Ok(Ok(0)) => ReadStatus::ConnectionClosed,
            Ok(Ok(_n)) => {
                // Process the header as a line containing the body length
                let body_len: usize = match read_header_buffer.trim().parse() {
                    Ok(v) => v,
                    Err(e) => {
                        debug!("failed to read header length: {e}");
                        return ReadStatus::Error("header parsing".to_string());
                    }
                };

                // Process the body
                read_body_buffer.resize(body_len, 0);
                if let Err(e) = reader.read_exact(read_body_buffer).await {
                    debug!("failed to read body: {e}");
                    return ReadStatus::Error("body parsing".to_string());
                };
                let body = String::from_utf8_lossy(read_body_buffer).into_owned();
                ReadStatus::Success(body)
            }
            Ok(Err(e)) => {
                debug!("failed to read from server: {e}");
                ReadStatus::Error("server".to_string())
            }
            Err(_) => ReadStatus::Timeout,
        }
    }
}

/// Models the possible outcomes when reading server responses
#[derive(Debug)]
enum ReadStatus {
    /// Successfully read data
    Success(String),

    /// Server connection was closed
    ConnectionClosed,

    /// Error occurred during reading
    Error(String),

    /// Initial read operation timed out
    Timeout,
}

impl Display for ReadStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadStatus::Success(_) => write!(f, "success"),
            ReadStatus::ConnectionClosed => write!(f, "connection closed"),
            ReadStatus::Error(e) => write!(f, "{e}"),
            ReadStatus::Timeout => write!(f, "timeout"),
        }
    }
}

struct StdinHandler {}

struct StdinHandle {
    tx: tokio::sync::broadcast::Sender<String>,
}

impl StdinHandler {
    fn start() -> StdinHandle {
        let (tx, _) = tokio::sync::broadcast::channel(64);
        let returned_tx = tx.clone();

        // Spawn a task to read from stdin
        tokio::spawn(async move {
            let mut stdin = BufReader::new(tokio::io::stdin());
            let mut read_buffer = String::new();

            loop {
                read_buffer.clear();
                match stdin.read_line(&mut read_buffer).await {
                    Ok(0) => break, // EOF (0 bytes read)
                    Ok(_n) => {
                        if read_buffer.trim().is_empty() {
                            continue;
                        }
                        if is_quit_sequence(&read_buffer) {
                            let _ = tx.send(read_buffer.clone());
                            drop(tx);
                            break;
                        }
                        let _ = tx.send(read_buffer.clone());
                    }
                    Err(e) => {
                        eprintln!("Error reading from stdin: {}", e);
                        break;
                    }
                }
            }
        });

        StdinHandle { tx: returned_tx }
    }
}

fn is_reconnect_sequence(input: &str) -> bool {
    input.trim() == ":reconnect" || input.trim() == ":r"
}

fn is_quit_sequence(input: &str) -> bool {
    input.trim() == ":quit" || input.trim() == ":q"
}

#[allow(dead_code)]
pub(super) mod dummy_server {
    use super::*;

    impl BaseCommand {
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

    #[test]
    fn test_get_address_for_outlet_with_port() -> miette::Result<()> {
        // Find a free port for testing
        let listener = TcpListener::bind("127.0.0.1:0").into_diagnostic()?;
        let test_port = listener.local_addr().into_diagnostic()?.port();
        // Drop listener to free the port
        drop(listener);
        std::thread::sleep(Duration::from_secs(1));

        // Create an outlet with a specific port
        let outlet = Outlet {
            name: Some("test-outlet".to_string()),
            to: format!("localhost:{}", test_port),
            ..Default::default()
        };

        let address = BaseCommand::get_address_for_inlet(&outlet)?;
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

        let address = BaseCommand::get_address_for_inlet(&outlet)?;
        assert_ne!(address.port(), occupied_port);

        Ok(())
    }
}
