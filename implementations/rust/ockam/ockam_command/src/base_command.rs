use crate::branding::BrandingCompileEnvVars;
use crate::cluster::common_args::HttpApiArgs;
use crate::zone::common_args::{DockerBuildArgs, ZoneInletsArgs};
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::watcher::DirectoryWatcher;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use ockam_api::fmt_warn;
use ockam_core::TryClone;
use ockam_node::Context;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,

    #[command(flatten)]
    pub docker_build: DockerBuildArgs,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    #[command(flatten)]
    pub inlets: ZoneInletsArgs,

    /// The name of the template project to download.
    /// It can be either a GitHub repository like `build-trust/ockam-cluster-template-hello`,
    /// a full URL like `git@github.com:build-trust/ockam-cluster-template-hello`,
    /// or an Ockam repository name that exists at `build-trust/ockam-cluster-template-<NAME>`
    #[arg(long, default_value = "hello", env = "INIT_REPOSITORY")]
    init_repository: String,

    /// Watch the current directory for changes and redeploy the zone when changes are detected.
    #[arg(long)]
    watch: bool,

    /// Remove the zone after the command is finished
    #[arg(long)]
    pub rm: bool,
}

impl BaseCommand {
    pub fn name(&self) -> String {
        BrandingCompileEnvVars::bin_name().to_string()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let res = tokio::select! {
            _ = ZoneCtrlcHandler::wait_for_message() => Ok(()),
            res = self.run_impl(ctx, &opts) => res,
        };
        if self.rm {
            let _ = self.delete_zone(ctx, &opts).await;
        }
        res
    }

    pub async fn run_impl(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        if self.watch {
            DirectoryWatcher::init()?;
            loop {
                let _ctx = ctx.try_clone()?;
                let _opts = opts.clone();
                tokio::select! {
                    res = DirectoryWatcher::wait_for_message() => {
                        let output = res?;
                        opts.terminal.clear_screen()?;
                        opts.terminal.write_line("\n".to_string() + &fmt_warn!("{output}\n"))?;
                    }
                    res = self.run_commands(&_ctx, &_opts) => {
                        res?;
                    },
                }
            }
        } else {
            self.run_commands(ctx, opts).await?;
        }
        Ok(())
    }

    async fn run_commands(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        self.enroll(ctx, opts).await?;
        self.zone_init(ctx, opts).await?;
        self.zone_deploy(ctx, opts).await?;
        self.zone_attach(opts, ctx).await?;
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
        use crate::zone::init::InitCommand;
        let cmd = InitCommand {
            repository: self.init_repository.clone(),
            ..Default::default()
        };
        if cmd.run(ctx, opts.clone()).await?.is_some() {
            opts.terminal.write_line("")?;
        }
        Ok(())
    }

    async fn zone_deploy(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        use crate::zone::deploy::DeployCommand;
        let cmd = DeployCommand {
            use_public_ecr: self.use_public_ecr,
            http_api: self.http_api.clone(),
            docker_build: self.docker_build.clone(),
            ..Default::default()
        };
        cmd.run(ctx, opts.clone()).await?;
        Ok(())
    }

    async fn zone_attach(&self, opts: &CommandGlobalOpts, ctx: &Context) -> Result<()> {
        use crate::zone::attach::AttachCommand;
        let cmd = AttachCommand {
            http_api: self.http_api.clone(),
            inlets: self.inlets.clone(),
            ..Default::default()
        };
        cmd.run_impl(opts.clone(), ctx).await
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
