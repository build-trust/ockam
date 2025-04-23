use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_api::fmt_separator;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_core::TryClone;
use ockam_node::Context;
use std::str::FromStr;
use tokio::task::JoinHandle;

#[derive(Clone, Debug, Args, Default)]
pub struct NoArgsCommand {}

impl NoArgsCommand {
    pub fn name(&self) -> String {
        "ockam".to_string()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, &opts).await?;
        self.cluster_init(ctx, &opts).await?;
        self.cluster_create(ctx, &opts).await?;
        let inlet_handle = self.cluster_inlet(ctx, &opts).await?;
        self.open_repl(ctx, &opts, inlet_handle).await?;

        Ok(())
    }

    async fn enroll(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let is_enrolled = opts.state.is_enrolled().await.is_ok_and(|v| v);
        if is_enrolled {
            return Ok(());
        }

        use crate::cluster::enroll::EnrollCommand;
        let enroll_command = EnrollCommand::default();
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
            repository: "build-trust/ockam-cluster-template-hello".to_string(),
            target_path: None,
        };
        init_command.run(ctx, opts.clone()).await?;
        opts.terminal.write_line(fmt_separator!())?;

        Ok(())
    }

    async fn cluster_create(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        use crate::cluster::create::CreateCommand;
        let create_command = CreateCommand {
            zone_name: "ockamtest".to_string(),
            use_public_ecr: true,
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        create_command.run(ctx, opts.clone()).await?;
        opts.terminal.write_line(fmt_separator!())?;

        Ok(())
    }

    async fn cluster_inlet(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<JoinHandle<Result<()>>> {
        use crate::cluster::ticket::TicketCommand;
        let ticket_command = TicketCommand {
            zone_name: "ockamtest".to_string(),
            api_endpoint: Some(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        let ticket = ticket_command.run(ctx, opts.clone()).await?;

        //TODO: this should be the pod name from the configuration.
        // Return needed data from cluster_create
        use crate::cluster::zone_config::ZoneConfig;
        let zone_config = ZoneConfig::from_file("./ockam.yaml")?;
        let pod_name = zone_config.pods.first().unwrap().name.clone();

        use crate::cluster::inlet::InletCommand;
        let inlet_command = InletCommand {
            zone_name: "ockamtest".to_string(),
            pod: pod_name,
            enrollment_ticket: ticket,
            from: SchemeHostnamePort::from_str("127.0.0.1:31234").into_diagnostic()?,
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
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpStream;

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

        loop {
            match TcpStream::connect("127.0.0.1:31234").await {
                Ok(_) => {
                    break;
                }
                Err(_) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        return Err(miette::miette!("Timed out waiting for inlet to be ready"));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        // Print the welcome message
        println!("Welcome to Ockam.\n");
        println!("You are connected to the inlet at 127.0.0.1:31234.\n");
        println!("Type :quit or :q to exit.");

        let (quit_tx, mut quit_rx) = tokio::sync::oneshot::channel();

        let input_task = tokio::spawn(async move {
            let mut stdin = BufReader::new(tokio::io::stdin());
            let mut buffer = String::new();

            loop {
                // Print the prompt
                print!("> ");
                io::stdout()
                    .flush()
                    .into_diagnostic()
                    .wrap_err("Failed to flush stdout")?;

                // Read a line
                buffer.clear();
                if stdin.read_line(&mut buffer).await.into_diagnostic()? == 0 {
                    // EOF reached
                    break;
                }

                let input = buffer.trim();

                // Check for quit commands
                if input == ":quit" || input == ":q" {
                    let _ = quit_tx.send(());
                    break;
                }

                if input.is_empty() {
                    continue;
                }

                // Try to connect to the inlet for each message
                match TcpStream::connect("127.0.0.1:31234").await {
                    Ok(mut stream) => {
                        let _ = stream.write_all(buffer.as_bytes()).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to inlet: {}", e);
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
        input_task.abort();

        Ok(())
    }
}
