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
use ockam_api::{fmt_ok, fmt_separator};
use ockam_core::env::get_env_ignore_error;
use ockam_core::TryClone;
use ockam_node::Context;
use std::io::{self, Write};
use std::str::FromStr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio::time::timeout;

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
        use tokio::time::{sleep, timeout};

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

        // Buffers
        let stdin = StdinHandler::start();

        // stdin quit handler
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

            async fn read_initial_message(
                reader: &mut tokio::net::tcp::OwnedReadHalf,
                read_buffer: &mut [u8],
            ) -> miette::Result<bool> {
                match timeout(Duration::from_secs(5), async {
                    loop {
                        match reader.read(read_buffer).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let chunk = String::from_utf8_lossy(&read_buffer[..n]).into_owned();
                                print!("{}", chunk);
                                io::stdout().flush().into_diagnostic()?;

                                // Exit if we received a partial buffer (n < buffer size)
                                if n < read_buffer.len() {
                                    break;
                                }
                            }
                            Err(e) => {
                                return Err(miette!("Error reading from server: {}", e));
                            }
                        }
                    }
                    Ok::<(), miette::Error>(())
                })
                .await
                {
                    Ok(Ok(())) => Ok(true),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Ok(false), // Timeout occurred
                }
            }

            let mut read_buffer = [0u8; 1024];

            'repl: loop {
                // 1: Try to connect to the inlet
                let stream = match connect_to_inlet(&inlet_address).await {
                    Ok(s) => s,
                    Err(_e) => {
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                };

                let (mut reader, mut writer) = stream.into_split();

                // 2: Try to read initial message
                match read_initial_message(&mut reader, &mut read_buffer).await {
                    Ok(true) => {
                        // Successfully got initial message, continue to REPL
                    }
                    _ => {
                        sleep(Duration::from_secs(1)).await;
                        continue 'repl;
                    }
                }

                // 3: Start the REPL loop for this connection
                'connection: loop {
                    io::stdout()
                        .flush()
                        .into_diagnostic()
                        .wrap_err("Failed to flush stdout")?;

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
                    if let Err(_e) = writer.write_all(user_input.as_bytes()).await {
                        println!("Failed to write to server. Reconnecting...");
                        break 'connection;
                    }

                    // Process response with streaming output
                    if Self::process_server_response(&mut reader, &mut read_buffer, &opts)
                        .await
                        .is_err()
                    {
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
        reader: &mut tokio::net::tcp::OwnedReadHalf,
        read_buffer: &mut [u8],
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        match Self::print_server_response(reader, read_buffer, Duration::from_secs(30)).await? {
            ReadStatus::ConnectionClosed => {
                println!("\nServer connection closed. Reconnecting...");
                Err(miette!("Server connection closed"))
            }
            ReadStatus::Success => Ok(()),
            ReadStatus::IoError(e) => {
                println!("\nError reading from server: {}", e);
                Err(miette!("Error reading from server: {}", e))
            }
            ReadStatus::Timeout => {
                // Short timeout reached, notify but keep waiting
                let spinner = opts.terminal.spinner();
                if let Some(spinner) = &spinner {
                    spinner.set_message("Waiting for server response...");
                }

                // Continue with longer timeout
                let res =
                    Self::print_server_response(reader, read_buffer, Duration::from_secs(5 * 60))
                        .await;

                if let Some(spinner) = &spinner {
                    spinner.finish_and_clear();
                }

                match res? {
                    ReadStatus::ConnectionClosed => {
                        println!("\nServer connection closed. Reconnecting...");
                        Err(miette!("Server connection closed"))
                    }
                    ReadStatus::Success => Ok(()),
                    ReadStatus::IoError(e) => {
                        println!("\nError reading from server: {}", e);
                        Err(miette!("Error reading from server: {}", e))
                    }
                    ReadStatus::Timeout => {
                        println!("\nServer response timeout. Reconnecting...");
                        Err(miette!("Server response timeout"))
                    }
                }
            }
        }
    }

    async fn print_server_response(
        reader: &mut tokio::net::tcp::OwnedReadHalf,
        read_buffer: &mut [u8],
        timeout_duration: Duration,
    ) -> Result<ReadStatus> {
        // Use timeout only for the initial read
        Ok(
            match timeout(timeout_duration, reader.read(read_buffer)).await {
                Ok(Ok(0)) => ReadStatus::ConnectionClosed,
                Ok(Ok(n)) => {
                    // Process the first chunk
                    let chunk = String::from_utf8_lossy(&read_buffer[0..n]).into_owned();
                    print!("{}", chunk);
                    io::stdout().flush().into_diagnostic()?;

                    if !is_server_end_sequence(&chunk) {
                        // Read more data
                        loop {
                            match reader.read(read_buffer).await {
                                Ok(0) => return Ok(ReadStatus::ConnectionClosed),
                                Ok(n) => {
                                    let chunk =
                                        String::from_utf8_lossy(&read_buffer[0..n]).into_owned();
                                    print!("{}", chunk);
                                    io::stdout().flush().into_diagnostic()?;
                                    // Check for end sequence
                                    if is_server_end_sequence(&chunk) {
                                        break;
                                    }
                                }
                                Err(e) => return Ok(ReadStatus::IoError(e)),
                            }
                        }
                    }

                    ReadStatus::Success
                }
                Ok(Err(e)) => ReadStatus::IoError(e),
                Err(_) => ReadStatus::Timeout,
            },
        )
    }

    #[allow(dead_code)]
    async fn dummy_inlet(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<JoinHandle<Result<()>>> {
        use tokio::net::TcpListener;

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Starting dummy echo inlet on {}...",
                color_primary(&self.inlet_address)
            ));
        }

        // Start a TCP server that echoes back any input
        let addr = self.inlet_address.hostname_port().to_string();
        let inlet_handle = tokio::spawn(async move {
            // Create TCP listener
            let listener = TcpListener::bind(&addr)
                .await
                .into_diagnostic()
                .wrap_err(format!("Failed to bind to {}", addr))?;

            loop {
                // Accept connections
                let (socket, _) = listener
                    .accept()
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to accept connection")?;

                // Handle each connection in a separate task
                tokio::spawn(async move {
                    if let Err(err) = Self::handle_connection(socket).await {
                        eprintln!("Connection error: {:?}", err);
                    }
                });
            }
        });

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Dummy echo inlet started on {}",
            color_primary(&self.inlet_address)
        ))?;

        Ok(inlet_handle)
    }

    // Helper function to handle individual connections
    #[allow(dead_code)]
    async fn handle_connection(mut socket: tokio::net::TcpStream) -> miette::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Send welcome message
        let welcome =
            "Welcome to the dummy echo inlet. Type anything and it will be echoed back.\n";
        socket
            .write_all(welcome.as_bytes())
            .await
            .into_diagnostic()?;

        let mut buffer = [0; 8192];

        // Read from socket in a loop
        loop {
            // Read data from socket
            let n = match socket.read(&mut buffer).await {
                Ok(0) => return Ok(()), // Connection closed
                Ok(n) => n,
                Err(e) => return Err(miette!("Failed to read from socket: {}", e)),
            };

            // Echo data back prefixed with "Echo: "
            let received = String::from_utf8_lossy(&buffer[..n]);
            let response = format!("Echo: {}\n", received.trim_end());

            // Write response back to client
            socket
                .write_all(response.as_bytes())
                .await
                .into_diagnostic()
                .wrap_err("Failed to write to socket")?;
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
    IoError(io::Error),

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
            let mut stdin = tokio::io::stdin();
            let mut read_buffer = [0u8; 1024];
            let mut accumulated_input = Vec::new();

            loop {
                match stdin.read(&mut read_buffer).await {
                    Ok(0) => break, // EOF (0 bytes read)
                    Ok(n) => {
                        // Append the new input to our accumulated buffer
                        accumulated_input.extend_from_slice(&read_buffer[..n]);

                        // Send the data if the input ends with a newline - TODO: doesn't support multiline input
                        if accumulated_input.ends_with(b"\n") {
                            let input = String::from_utf8_lossy(&accumulated_input).into_owned();
                            if is_quit_sequence(&input) {
                                let _ = tx.send(input);
                                drop(tx);
                                break;
                            }
                            let _ = tx.send(input);
                            // Clear the accumulated input after processing
                            accumulated_input.clear();
                        }
                        // If the message length is greater or equal than the buffer sie, continue accumulating
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

fn is_server_end_sequence(input: &str) -> bool {
    input.ends_with("\n\n> ")
}
