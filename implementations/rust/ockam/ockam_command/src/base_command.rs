use crate::branding::BrandingCompileEnvVars;
use crate::cluster::common_args::HttpApiArgs;
use crate::zone::common_args::ZoneConfigArg;
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::repl::ReplExitCondition;
use crate::zone::zone_config::ZoneConfig;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use notify::Watcher;
use ockam_api::fmt_warn;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_core::TryClone;
use ockam_node::Context;
use std::path::PathBuf;
use std::time::Duration;
use tracing::warn;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
    #[arg(long)]
    watch: bool,

    #[arg(long)]
    no_docker_cache: bool,

    #[arg(long, default_value = "hello", env = "INIT_REPOSITORY")]
    init_repository: String,

    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,

    /// Skip the creation of the inlet to the http outlet.
    #[arg(long)]
    no_http: bool,

    /// Skip the creation of the inlet to the logs outlet.
    #[arg(long)]
    no_logs: bool,

    /// Remove the zone after the command is finished
    #[arg(long)]
    pub rm: bool,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let mut quit_rx = ZoneCtrlcHandler::rx();
        let res = tokio::select! {
            _ = quit_rx.recv() => Ok(()),
            res = self.run_impl(ctx, &opts) => res,
        };
        if self.rm {
            let _ = self.delete_zone(ctx, &opts).await;
        }
        res
    }

    pub async fn run_impl(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let mut quit_rx = ZoneCtrlcHandler::rx();
        self.enroll(ctx, opts).await?;
        self.zone_init(ctx, opts).await?;
        if !self.watch {
            self.zone_create(ctx, opts).await?;
            self.zone_repl(opts, None).await?;
        } else {
            let watcher_handle = DirectoryWatcher::run(
                std::env::current_dir()
                    .into_diagnostic()
                    .wrap_err("Failed to get current directory")?,
            )?;
            loop {
                let restart_tx = watcher_handle.tx.clone();
                let _self = self.clone();
                let _ctx = ctx.try_clone()?;
                let _opts = opts.clone();
                let run_zone_handle = tokio::spawn(async move {
                    _self.zone_create(&_ctx, &_opts).await?;
                    let res = _self.zone_repl(&_opts, Some(restart_tx)).await?;
                    Ok::<ReplExitCondition, miette::Error>(res)
                });
                tokio::select! {
                    _ = quit_rx.recv() => {
                        break;
                    }
                    res = run_zone_handle => {
                        match res {
                            Ok(Ok(ReplExitCondition::Exit)) => break,
                            Ok(Ok(ReplExitCondition::Restart)) => continue,
                            Ok(Err(e)) => {
                                return Err(e);
                            }
                            Err(e) => {
                                return Err(miette!(e));
                            }
                        }
                    },
                    result = watcher_handle.recv() => {
                        let output = result?;
                        opts.terminal.write_line("\n".to_string() + &fmt_warn!("{output}\n"))?;
                        continue;
                    }
                }
            }
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

    async fn zone_init(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let current_dir = std::env::current_dir()
            .into_diagnostic()
            .wrap_err("Failed to get current directory")?;
        if current_dir.read_dir().into_diagnostic()?.next().is_some() {
            return Ok(());
        }

        use crate::zone::init::InitCommand;
        let cmd = InitCommand {
            repository: self.init_repository.clone(),
            target_path: None,
        };
        cmd.run(ctx, opts.clone()).await?;
        opts.terminal.write_line("")?;

        Ok(())
    }

    async fn zone_create(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<ZoneConfig> {
        use crate::zone::create::CreateCommand;
        let cmd = CreateCommand {
            use_public_ecr: self.use_public_ecr,
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            no_docker_cache: self.no_docker_cache,
            ..Default::default()
        };
        let zone_config = cmd.run(ctx, opts.clone()).await?;
        Ok(zone_config)
    }

    async fn zone_repl(
        &self,
        opts: &CommandGlobalOpts,
        restart_tx: Option<tokio::sync::broadcast::Sender<String>>,
    ) -> Result<ReplExitCondition> {
        use crate::zone::repl::ReplCommand;
        let cmd = ReplCommand {
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            no_http: self.no_http,
            no_logs: self.no_logs,
            ..Default::default()
        };
        cmd.run_impl(opts.clone(), restart_tx).await
    }

    async fn delete_zone(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        use crate::zone::delete::DeleteCommand;
        let cmd = DeleteCommand {
            yes: true,
            ..Default::default()
        };
        cmd.run(ctx, opts.clone()).await?;
        Ok(())
    }
}

struct DirectoryWatcher {
    watcher: notify::RecommendedWatcher,
    tx: tokio::sync::broadcast::Sender<notify::Event>,
    root_dir: PathBuf,
    zone_config_file_name: String,
}

impl DirectoryWatcher {
    fn run(root_dir: PathBuf) -> Result<DirectoryWatcherHandle> {
        let (watcher_tx, _rx) = tokio::sync::broadcast::channel(16);
        let _tx = watcher_tx.clone();
        let watcher = notify::RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = _tx.send(event);
                }
            },
            notify::Config::default(),
        )
        .into_diagnostic()?;
        let zone_config_file_name = ZoneConfigArg::default()
            .zone_config_path()?
            .file_name()
            .ok_or_else(|| miette!("Invalid zone config file name"))?
            .to_string_lossy()
            .to_string();
        let _self = Self {
            watcher,
            tx: watcher_tx,
            root_dir,
            zone_config_file_name,
        };
        let (restart_tx, _rx) = tokio::sync::broadcast::channel(16);
        let _restart_tx = restart_tx.clone();
        tokio::task::spawn(async move {
            let _ = _self._run(_restart_tx).await;
        });
        Ok(DirectoryWatcherHandle { tx: restart_tx })
    }

    async fn _run(
        mut self,
        restart_tx: tokio::sync::broadcast::Sender<String>,
    ) -> miette::Result<()> {
        self.watcher
            .watch(self.root_dir.as_ref(), notify::RecursiveMode::Recursive)
            .into_diagnostic()?;
        let mut watcher_rx = self.tx.subscribe();

        let debounce_period = Duration::from_secs(10);
        let mut pending_message: Option<String> = None;
        let mut debounce_timer: Option<tokio::time::Instant> = None;

        let images_path = self.root_dir.join("images");
        loop {
            tokio::select! {
                event_result = watcher_rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            let mut should_restart = false;

                            for path in &event.paths {
                                // Check for modifications of the zone config file
                                if path.file_name().is_some_and(|name| {
                                    name.to_string_lossy().as_ref() == self.zone_config_file_name
                                }) && path.parent().is_some_and(|parent| parent == self.root_dir)
                                {
                                    if let notify::EventKind::Modify(_) = event.kind {
                                        pending_message = Some("Detected changes in the zone config file".to_string());
                                        should_restart = true;
                                        break;
                                    }
                                }
                                // Check for creation or modification of files in the images directory
                                if path.starts_with(&images_path) {
                                    if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind
                                    {
                                        pending_message = Some("Detected changes in the images directory".to_string());
                                        should_restart = true;
                                        break;
                                    }
                                }
                            }

                            if should_restart {
                                // Reset the countdown timer when new changes are detected
                                debounce_timer = Some(tokio::time::Instant::now() + debounce_period);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to receive file system event: {}", e);
                            break;
                        }
                    }
                },
                // Check if it's time to send the restart signal
                _ = async {
                    if let Some(timer) = debounce_timer {
                        tokio::time::sleep_until(timer).await;
                        Ok::<(), miette::Error>(())
                    } else {
                        // If there's no timer set, this branch will never complete
                        std::future::pending::<()>().await;
                        Ok(())
                    }
                } => {
                    if let Some(message) = pending_message.take() {
                        if let Err(e) = restart_tx.send(message) {
                            warn!(%e, "failed to send restart signal");
                            break;
                        }
                    }
                    // Reset the timer after sending
                    debounce_timer = None;
                }
            }
        }
        Ok(())
    }
}

struct DirectoryWatcherHandle {
    tx: tokio::sync::broadcast::Sender<String>,
}

impl DirectoryWatcherHandle {
    async fn recv(&self) -> Result<String> {
        let output = self.tx.subscribe().recv().await.into_diagnostic()?;
        Ok(output)
    }
}
