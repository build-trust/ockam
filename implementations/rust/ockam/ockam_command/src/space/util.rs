use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::cloud::space::Space;
use ockam_multiaddr::MultiAddr;

use crate::util::{api, RpcBuilder};
use crate::{CommandGlobalOpts, Result};

async fn refresh_spaces(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    controller_route: &MultiAddr,
) -> Result<()> {
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();
    rpc.request(api::space::list(controller_route)).await?;
    let spaces = rpc.parse_response::<Vec<Space>>()?;
    for space in spaces {
        opts.state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
    }
    Ok(())
}
