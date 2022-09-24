use clap::Args;
use rand::prelude::random;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::util::delete_embedded_node;
use crate::space::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use colorful::Colorful;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space.
    #[arg(display_order = 1001, default_value_t = hex::encode(&random::<[u8;4]>()), hide_default_value = true)]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    /// Administrators for this space
    #[arg(display_order = 1100, last = true)]
    pub admins: Vec<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        println!(
            "\n{}",
            "Creating a trial space for you (everything in it will be deleted in 15 days) ..."
                .light_magenta()
        );
        println!(
            "{}",
            "To learn more about production ready spaces in Ockam Orchestrator, contact us at: hello@ockam.io".light_magenta()
        );
        node_rpc(rpc, (options, self));
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
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::space::create(&cmd)).await?;
    let space = rpc.parse_and_print_response::<Space>()?;
    config::set_space(&opts.config, &space)?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
