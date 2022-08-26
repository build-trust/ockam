use clap::Args;
use ockam::{Context, TcpTransport};
use std::io::Write;

use ockam_api::cloud::project::Project;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Id of the space the project belongs to.
    #[clap(display_order = 1001)]
    pub space_id: String,

    /// Name of the project.
    #[clap(display_order = 1002)]
    pub project_name: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    /// Services enabled for this project.
    #[clap(display_order = 1100, last = true)]
    pub services: Vec<String>,
    //TODO:  list of admins
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> crate::Result<()> {
    let tcp = TcpTransport::create(ctx).await?;
    let mut rpc = RpcBuilder::new(ctx, &opts, &cmd.node_opts.api_node)
        .tcp(&tcp)
        .build()?;
    rpc.request(api::project::create(&cmd)).await?;
    let mut project = rpc.parse_response::<Project>()?;

    if project.access_route.is_empty() {
        print!("\nProject created. Waiting until it's operative...");
        let cmd = crate::project::ShowCommand {
            space_id: project.space_id.to_string(),
            project_id: project.id.to_string(),
            node_opts: cmd.node_opts.clone(),
            cloud_opts: cmd.cloud_opts.clone(),
        };
        loop {
            print!(".");
            std::io::stdout().flush()?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let mut rpc = RpcBuilder::new(ctx, &opts, &cmd.node_opts.api_node)
                .tcp(&tcp)
                .build()?;
            rpc.request(api::project::show(&cmd)).await?;
            let p = rpc.parse_response::<Project>()?;
            if p.is_ready() {
                project = p.to_owned();
                break;
            }
        }
    }
    opts.config.set_project_alias(
        project.name.to_string(),
        project.access_route.to_string(),
        project.id.to_string(),
        project
            .identity
            .as_ref()
            .expect("Project should have identity set")
            .to_string(),
    )?;
    println!("{}", project.output()?);
    Ok(())
}
