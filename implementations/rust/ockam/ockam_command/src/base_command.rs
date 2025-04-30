use crate::branding::BrandingCompileEnvVars;
use crate::cluster::zone_config::ZoneConfig;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::colors::color_primary;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_api::{fmt_ok, fmt_separator};
use ockam_core::env::get_env_ignore_error;
use ockam_core::TryClone;
use ockam_node::Context;
use std::str::FromStr;
use std::time::Duration;
use tokio::task::JoinHandle;

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
        self.inlet_address = SchemeHostnamePort::from_str("127.0.0.1:31234")?;

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
        _opts: &CommandGlobalOpts,
        inlet_handle: JoinHandle<Result<()>>,
    ) -> miette::Result<()> {
        use std::io::{self, Write};
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpStream;
        use tokio::time::{sleep, timeout};

        // Inlet monitor future
        let (inlet_tx, inlet_rx) = tokio::sync::oneshot::channel::<()>();
        let _inlet_monitor = async move {
            match inlet_handle.await {
                Ok(result) => result,
                Err(err) => {
                    let _ = inlet_tx.send(());
                    Err(miette!("{err:?}"))
                }
            }
        };

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

        async fn read_message(
            reader: &mut tokio::net::tcp::OwnedReadHalf,
            timeout_duration: Duration,
        ) -> miette::Result<String> {
            let mut reader = BufReader::new(reader);

            // Read the header line
            let mut header_line = String::new();
            return match timeout(timeout_duration, reader.read_line(&mut header_line))
                .await
                .into_diagnostic()?
            {
                Err(_) => Err(miette!("Failed to read the header line")),
                Ok(0) => Err(miette!("Connection closed while reading the header line")),
                Ok(_) => {
                    let header_line = header_line.trim();

                    // Parse the header line as an integer
                    let body_size: usize = header_line
                        .parse()
                        .into_diagnostic()
                        .wrap_err("Failed to parse header line as an integer")?;

                    // Read the body with the size given by the header
                    let mut body = vec![0u8; body_size];
                    reader.read_exact(&mut body).await.into_diagnostic()?;
                    std::str::from_utf8(&body)
                        .map(|s| s.to_owned())
                        .into_diagnostic()
                }
            };
        }

        async fn repl_loop(inlet_addr: &SchemeHostnamePort) -> miette::Result<()> {
            let mut stdin = BufReader::new(tokio::io::stdin());
            let stream = connect_to_inlet(inlet_addr).await?;

            let (mut reader, mut writer) = stream.into_split();
            let msg = read_message(&mut reader, Duration::from_secs(10)).await?;
            print!("{}", msg);
            io::stdout().flush().into_diagnostic()?;
            loop {
                let mut string_buffer = String::new();
                if stdin
                    .read_line(&mut string_buffer)
                    .await
                    .into_diagnostic()?
                    == 0
                {
                    // EOF reached
                    break;
                }

                let input = string_buffer.trim();

                // Check for quit commands
                if input == ":quit" || input == ":q" {
                    break;
                }
                if !input.is_empty() {
                    // Send command to server
                    if let Err(e) = writer.write_all(string_buffer.as_bytes()).await {
                        println!("Failed to write to server. Reconnecting...");
                        return Err(e).into_diagnostic();
                    }
                    let reply = read_message(&mut reader, Duration::from_secs(5 * 60)).await?;
                    print!("{}", reply);
                    io::stdout().flush().into_diagnostic()?;
                }
            }
            return Ok(());
        }

        let (quit_tx, mut quit_rx) = tokio::sync::oneshot::channel();

        // Start the interactive REPL
        let inlet_address = self.inlet_address.clone();
        let repl_handle = tokio::spawn(async move {
            loop {
                match repl_loop(&inlet_address).await {
                    Ok(()) => {
                        let _ = quit_tx.send(());
                        break;
                    }
                    Err(_) => {
                        println!("Repl Connection Lost. Reconnecting...");
                        // If we break from the connection loop, we'll go back to connecting
                        sleep(Duration::from_secs(1)).await
                    }
                }
            }
        });

        // Wait for exit signals
        tokio::select! {
            _ = &mut quit_rx => {},
            _result = inlet_rx => {},
        }

        // Clean up tasks
        repl_handle.abort();

        Ok(())
    }
}
