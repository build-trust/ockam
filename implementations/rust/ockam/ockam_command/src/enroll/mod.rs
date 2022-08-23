use clap::Args;

pub use auth0::Auth0Service;
use auth0::*;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::CommandGlobalOpts;

mod auth0;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl EnrollCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: EnrollCommand) {
        EnrollAuth0Command::run(opts, cmd)
    }
}
