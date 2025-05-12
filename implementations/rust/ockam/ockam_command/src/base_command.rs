use crate::branding::BrandingCompileEnvVars;
use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::zone_config::ZoneConfig;
use crate::{Command, CommandGlobalOpts, Result};
use clap::Args;
use miette::{IntoDiagnostic, WrapErr};
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL;
use ockam_node::Context;

#[derive(Clone, Debug, Args, Default)]
pub struct BaseCommand {
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
        cmd.cluster_create(ctx, &opts).await?;
        cmd.cluster_attach(ctx, &opts).await?;
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

    async fn cluster_attach(self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<()> {
        use crate::cluster::attach::AttachCommand;
        let attach_command = AttachCommand {
            http_api: HttpApiArgs::from_api_endpoint(AI_API_BASE_URL.to_string()),
            ..Default::default()
        };
        attach_command.run(ctx, opts.clone()).await
    }
}
