use crate::branding::BrandingCompileEnvVars;
use crate::cluster::zone_config::ZoneConfig;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::colors::color_primary;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_api::{fmt_log, fmt_separator};
use ockam_core::env::get_env_ignore_error;
use ockam_core::TryClone;
use ockam_node::Context;
use std::str::FromStr;
use tokio::task::JoinHandle;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
    init_repository: String,
    zone_name: String,
    inlet_address: SchemeHostnamePort,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> Result<Self> {
        // load default values
        self.init_repository = "hello".to_string();
        self.zone_name = "".to_string();
        self.inlet_address = SchemeHostnamePort::from_str("127.0.0.1:31234")?;

        // load env vars
        if let Some(v) = get_env_ignore_error("INIT_REPOSITORY") {
            self.init_repository = v;
        }
        if let Some(v) = get_env_ignore_error("ZONE_NAME") {
            self.zone_name = v;
        }
        if let Some(v) = get_env_ignore_error::<String>("INLET_ADDRESS") {
            self.inlet_address = v.parse().into_diagnostic()?;
        }

        // process default value for zone_name
        if self.zone_name.is_empty() {
            let user_info = opts.state.get_default_user().await?;
            self.zone_name = hex::encode(user_info.email.to_string());
            opts.terminal.write_line(fmt_log!(
                "Using zone name {}",
                color_primary(&self.zone_name)
            ))?;
        }

        Ok(self)
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;
        cmd.cluster_init(ctx, &opts).await?;
        let zone_config = cmd.cluster_create(ctx, &opts).await?;
        let inlet_handle = cmd.cluster_inlet(ctx, &opts, &zone_config).await?;
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
        opts.terminal.write_line(fmt_separator!())?;

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
        opts.terminal.write_line(fmt_separator!())?;

        Ok(())
    }

    async fn cluster_create(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<ZoneConfig> {
        use crate::cluster::create::CreateCommand;
        let create_command = CreateCommand {
            zone_name: self.zone_name.clone(),
            use_public_ecr: true,
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let zone_config = create_command.run(ctx, opts.clone()).await?;
        opts.terminal.write_line(fmt_separator!())?;

        Ok(zone_config)
    }

    async fn cluster_inlet(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<JoinHandle<Result<()>>> {
        use crate::cluster::ticket::TicketCommand;
        let ticket_command = TicketCommand {
            zone_name: self.zone_name.clone(),
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let ticket = ticket_command.run(ctx, opts.clone()).await?;

        let pod_name = zone_config
            .pods
            .first()
            .ok_or_else(|| miette!("No pods found in the parsed zone configuration"))?
            .name
            .clone();

        use crate::cluster::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone_name: self.zone_name.clone(),
            pod: pod_name,
            enrollment_ticket: ticket,
            from: self.inlet_address.clone(),
            api_endpoint: None,
            ..Default::default()
        };
        let ctx = ctx.try_clone()?;
        let opts = opts.clone();
        let handle = tokio::spawn(async move { inlet_command.run(&ctx, opts).await });

        Ok(handle)
    }

    async fn open_repl(
        &self,
        _ctx: &Context,
        _opts: &CommandGlobalOpts,
        inlet_handle: JoinHandle<Result<()>>,
    ) -> miette::Result<()> {
        use std::io::{self, Write};
        use std::time::Duration;
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

        // Wait until the inlet is ready
        const MAX_RETRIES: u32 = 120; // Try for about 60 seconds (120 * 500ms)
        let mut retries = 0;
        let mut stream = None;
        while retries < MAX_RETRIES {
            match TcpStream::connect(self.inlet_address.hostname_port().to_string()).await {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_) => {
                    retries += 1;
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
        let stream = match stream {
            Some(s) => s,
            None => return Err(miette!("Failed to connect to the TCP Inlet")),
        };

        let (quit_tx, mut quit_rx) = tokio::sync::oneshot::channel();
        let (mut reader, mut writer) = stream.into_split();

        // Read initial message from server
        let mut welcome_buf = Vec::new();
        let mut tmp_buf = [0u8; 1024];
        match timeout(Duration::from_secs(5), async {
            loop {
                match reader.read(&mut tmp_buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        welcome_buf.extend_from_slice(&tmp_buf[..n]);
                        // Check if we've received a complete message
                        if n < 1024 {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .await
        {
            Ok(_) => {
                if !welcome_buf.is_empty() {
                    if let Ok(msg) = String::from_utf8(welcome_buf) {
                        print!("{}", msg);
                        io::stdout().flush().into_diagnostic()?;
                    }
                }
            }
            Err(_) => {} // Timeout occurred, continue anyway
        }

        println!("Type :quit or :q to exit.");

        // Start the interactive REPL
        let repl_handle = tokio::spawn(async move {
            let mut stdin = BufReader::new(tokio::io::stdin());
            let mut string_buffer = String::new();
            let mut byte_buffer = [0u8; 1024];

            loop {
                // Print the prompt
                print!("> ");
                io::stdout()
                    .flush()
                    .into_diagnostic()
                    .wrap_err("Failed to flush stdout")?;

                // Send user input
                string_buffer.clear();
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

                if input == ":quit" || input == ":q" {
                    let _ = quit_tx.send(());
                    break;
                }

                if input.is_empty() {
                    continue;
                }

                if let Err(e) = writer.write_all(string_buffer.as_bytes()).await {
                    eprintln!("Failed to write to server: {e}");
                    break;
                }

                // Wait for a response
                match reader.read(&mut byte_buffer).await {
                    Ok(0) => {
                        // Connection closed
                        println!("\nServer connection closed.");
                        break;
                    }
                    Ok(n) => {
                        if let Ok(s) = std::str::from_utf8(&byte_buffer[..n]) {
                            print!("{}", s);
                            io::stdout().flush().into_diagnostic()?;
                        }
                    }
                    Err(e) => {
                        eprintln!("\nError reading from server: {}", e);
                        break;
                    }
                }
            }

            Ok::<(), miette::Error>(())
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
