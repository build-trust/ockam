use crate::branding::BrandingCompileEnvVars;
use crate::cluster::zone_config::ZoneConfig;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::address::get_free_address;
use ockam_api::colors::color_primary;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_api::terminal::Terminal;
use ockam_api::{fmt_ok, fmt_separator};
use ockam_core::env::get_env_ignore_error;
use ockam_core::TryClone;
use ockam_node::Context;
use std::str::FromStr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::debug;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
    init_repository: String,
    inlet_address: SchemeHostnamePort,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    async fn parse_args(mut self, _opts: &CommandGlobalOpts) -> Result<Self> {
        // load default values
        self.init_repository = "hello".to_string();
        self.inlet_address = {
            let address = get_free_address()?;
            SchemeHostnamePort::from_str(&address.to_string())?
        };

        // load env vars
        if let Some(v) = get_env_ignore_error("INIT_REPOSITORY") {
            self.init_repository = v;
        }
        if let Some(v) = get_env_ignore_error::<String>("INLET_ADDRESS") {
            self.inlet_address = v.parse().into_diagnostic()?;
        }

        Ok(self)
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;
        cmd.cluster_init(ctx, &opts).await?;
        let zone_config = cmd.cluster_create(ctx, &opts).await?;
        let inlet_handle = cmd.cluster_inlet(ctx, &opts, &zone_config).await?;
        // let inlet_handle = cmd.dummy_inlet(&opts).await?;
        opts.terminal.write_line(fmt_separator!())?;
        cmd.open_repl(ctx, &opts, inlet_handle).await?;
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
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let zone_config = create_command.run(ctx, opts.clone()).await?;
        Ok(zone_config)
    }

    async fn cluster_inlet(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<JoinHandle<Result<()>>> {
        let zone_name = zone_config.name.clone();
        let pod_name = zone_config
            .pods
            .first()
            .ok_or_else(|| miette!("No pods found in the parsed zone configuration"))?
            .name
            .clone();

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Opening a Portal to the outlet {} in {}...",
                color_primary(&pod_name),
                color_primary(&self.inlet_address)
            ));
        }

        // Disable terminal output for the following commands
        let mut no_output_opts = opts.clone();
        no_output_opts.terminal = opts.terminal.disable();

        use crate::cluster::ticket::TicketCommand;
        let ticket_command = TicketCommand {
            zone_name: zone_name.clone(),
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let ticket = ticket_command.run(ctx, no_output_opts.clone()).await?;

        use crate::cluster::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone_name: zone_name.clone(),
            pod: pod_name.clone(),
            enrollment_ticket: ticket,
            from: self.inlet_address.clone(),
            ..Default::default()
        };
        let ctx = ctx.try_clone()?;
        let handle = tokio::spawn(async move { inlet_command.run(&ctx, no_output_opts).await });

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Portal connected to the outlet {} in {}",
            color_primary(&pod_name),
            color_primary(&self.inlet_address)
        ))?;

        Ok(handle)
    }

    async fn open_repl(
        &self,
        _ctx: &Context,
        opts: &CommandGlobalOpts,
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
                    Err(miette!("{err:?}"))
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
        let inlet_address = self.inlet_address.clone();
        let mut stdin_rx = stdin.tx.subscribe();
        let repl_handle = tokio::spawn(async move {
            async fn connect_to_inlet(addr: &SchemeHostnamePort) -> miette::Result<TcpStream> {
                const MAX_RETRIES: u32 = 120; // Try for about 60 seconds (120 * 500ms)
                let mut retries = 0;

                while retries < MAX_RETRIES {
                    match TcpStream::connect(addr.hostname_port().to_string()).await {
                        Ok(s) => return Ok(s),
                        Err(_) => {
                            retries += 1;
                            sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
                Err(miette!("Failed to connect to the TCP Inlet"))
            }

            let mut read_header_buffer = String::new();
            let mut read_body_buffer = Vec::new();

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
                match Self::process_server_response(
                    &opts,
                    &mut reader,
                    &mut read_header_buffer,
                    &mut read_body_buffer,
                )
                .await
                {
                    Ok(()) => {
                        // Successfully got initial message, continue to REPL
                    }
                    Err(e) => {
                        debug!("Failed to read from server: {:?}", e);
                        opts.terminal
                            .write_line("Failed to read from server. Reconnecting...")?;
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                }

                // 3: Start the REPL loop for this connection
                'connection: loop {
                    Terminal::flush(&opts.terminal)?;

                    // Get user input
                    let user_input = match stdin_rx.recv().await {
                        Ok(i) => i,
                        Err(_) => {
                            // stdin channel closed, exit REPL
                            break 'repl;
                        }
                    };

                    if user_input.is_empty() {
                        continue 'connection;
                    }

                    if is_quit_sequence(&user_input) {
                        break 'repl;
                    }

                    // Send input to server
                    if let Err(e) = writer.write_all(user_input.as_bytes()).await {
                        debug!("Failed to write to server: {:?}", e);
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
                        debug!("Failed to read from server: {:?}", e);
                        opts.terminal
                            .write_line("Failed to read from server. Reconnecting...")?;
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

    async fn process_server_response(
        opts: &CommandGlobalOpts,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
    ) -> miette::Result<()> {
        match Self::print_server_response(
            opts,
            reader,
            read_header_buffer,
            read_body_buffer,
            Duration::from_secs(30),
        )
        .await?
        {
            ReadStatus::ConnectionClosed => Err(miette!("Server connection closed")),
            ReadStatus::Success => Ok(()),
            ReadStatus::IoError(e) => Err(miette!(e).wrap_err("Error reading from server")),
            ReadStatus::Timeout => {
                // Short timeout reached, notify but keep waiting
                let spinner = opts.terminal.spinner();
                if let Some(spinner) = &spinner {
                    spinner.set_message("Waiting for server response...");
                }

                // Continue with longer timeout
                let res = Self::print_server_response(
                    opts,
                    reader,
                    read_header_buffer,
                    read_body_buffer,
                    Duration::from_secs(5 * 60),
                )
                .await;

                if let Some(spinner) = &spinner {
                    spinner.finish_and_clear();
                }

                match res? {
                    ReadStatus::ConnectionClosed => Err(miette!("Server connection closed")),
                    ReadStatus::Success => Ok(()),
                    ReadStatus::IoError(e) => Err(miette!(e).wrap_err("Error reading from server")),
                    ReadStatus::Timeout => Err(miette!("Server response timeout")),
                }
            }
        }
    }

    async fn print_server_response(
        opts: &CommandGlobalOpts,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        read_header_buffer: &mut String,
        read_body_buffer: &mut Vec<u8>,
        timeout_duration: Duration,
    ) -> Result<ReadStatus> {
        // Use timeout only for the header read
        read_header_buffer.clear();
        match timeout(timeout_duration, reader.read_line(read_header_buffer)).await {
            Ok(Ok(0)) => Ok(ReadStatus::ConnectionClosed),
            Ok(Ok(_n)) => {
                // Process the header as a line containing the body length
                let body_len: usize = read_header_buffer
                    .trim()
                    .parse()
                    .into_diagnostic()
                    .wrap_err(format!(
                        "Failed to parse header line as an integer: {read_header_buffer}"
                    ))?;

                // Process the body
                read_body_buffer.resize(body_len, 0);
                reader
                    .read_exact(read_body_buffer)
                    .await
                    .into_diagnostic()?;
                let body = String::from_utf8_lossy(read_body_buffer).into_owned();
                opts.terminal.write(&body)?;
                Ok(ReadStatus::Success)
            }
            Ok(Err(e)) => Ok(ReadStatus::IoError(e)),
            Err(_) => Ok(ReadStatus::Timeout),
        }
    }
}

/// Models the possible outcomes when reading server responses
#[derive(Debug)]
enum ReadStatus {
    /// Successfully read data
    Success,

    /// Server connection was closed
    ConnectionClosed,

    /// Error occurred during reading
    IoError(std::io::Error),

    /// Initial read operation timed out
    Timeout,
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
                match stdin.read_line(&mut read_buffer).await {
                    Ok(0) => break, // EOF (0 bytes read)
                    Ok(_n) => {
                        if is_quit_sequence(&read_buffer) {
                            let _ = tx.send(read_buffer.clone());
                            drop(tx);
                            break;
                        }
                        let _ = tx.send(read_buffer.clone());
                        read_buffer.clear();
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
        ) -> miette::Result<JoinHandle<Result<()>>> {
            use tokio::net::TcpListener;

            let addr = self.inlet_address.hostname_port().to_string();
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
                color_primary(&self.inlet_address)
            ))?;

            Ok(inlet_handle)
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
