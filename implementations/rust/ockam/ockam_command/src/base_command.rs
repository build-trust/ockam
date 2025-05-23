use crate::branding::BrandingCompileEnvVars;
use crate::cluster::common_args::{HttpApiArgs, ZoneConfigArg};
use crate::cluster::ctrlc::ClusterCtrlcHandler;
use crate::cluster::zone_config::ZoneConfig;
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
    ignore_docker_cache: bool,

    #[arg(long, default_value = "hello", env = "INIT_REPOSITORY")]
    init_repository: String,

    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    async fn parse_args(self, _opts: &CommandGlobalOpts) -> Result<Self> {
        Ok(self)
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;
        cmd.cluster_init(ctx, &opts).await?;
        if !cmd.watch {
            cmd.cluster_create(ctx, &opts).await?;
            cmd.cluster_attach(&opts, None).await?;
        } else {
            let watcher_handle = DirectoryWatcher::run(
                std::env::current_dir()
                    .into_diagnostic()
                    .wrap_err("Failed to get current directory")?,
            )?;
            loop {
                let mut quit_rx = ClusterCtrlcHandler::rx();
                let restart_tx = watcher_handle.tx.clone();
                let _cmd = cmd.clone();
                let _ctx = ctx.try_clone()?;
                let _opts = opts.clone();
                let run_cluster_handle = tokio::spawn(async move {
                    _cmd.cluster_create(&_ctx, &_opts).await?;
                    _cmd.cluster_attach(&_opts, Some(restart_tx)).await?;
                    Ok::<(), miette::Error>(())
                });
                tokio::select! {
                    _ = quit_rx.recv() => {
                        break;
                    }
                    _ = run_cluster_handle => {
                        break;
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
            use_public_ecr: self.use_public_ecr,
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            ignore_docker_cache: self.ignore_docker_cache,
            ..Default::default()
        };
        let zone_config = create_command.run(ctx, opts.clone()).await?;
        Ok(zone_config)
    }

    async fn cluster_attach(
        self,
        opts: &CommandGlobalOpts,
        restart_tx: Option<tokio::sync::broadcast::Sender<String>>,
    ) -> Result<()> {
        use crate::cluster::attach::AttachCommand;
        let attach_command = AttachCommand {
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        attach_command.run_impl(opts.clone(), restart_tx).await
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

        let mut last_send_time = std::time::Instant::now() - Duration::from_secs(10);
        let debounce_period = Duration::from_secs(1);

        let mut message = None;
        let images_path = self.root_dir.join("images");
        while let Ok(event) = watcher_rx.recv().await {
            let now = std::time::Instant::now();
            for path in &event.paths {
                // Check for modifications of the zone config file
                if path.file_name().is_some_and(|name| {
                    name.to_string_lossy().as_ref() == self.zone_config_file_name
                }) && path.parent().is_some_and(|parent| parent == self.root_dir)
                {
                    if let notify::EventKind::Modify(_) = event.kind {
                        message = Some("Detected changes in the zone config file".to_string());
                        break;
                    }
                }
                // Check for creation or modification of files in the images directory
                if path.starts_with(&images_path) {
                    if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind
                    {
                        message = Some("Detected changes in the images directory".to_string());
                        break;
                    }
                }
            }
            if let Some(message) = message.take() {
                if now.duration_since(last_send_time) >= debounce_period {
                    if let Err(e) = restart_tx.send(message) {
                        warn!(%e, "failed to send restart signal");
                        break;
                    }
                    last_send_time = now;
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
