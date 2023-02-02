use anyhow::{Context as _, Result};

use ockam::Context;
use ockam_api::cloud::space::Space;

pub mod config {
    use crate::util::{api, RpcBuilder};
    use crate::CommandGlobalOpts;
    use ockam::TcpTransport;
    use ockam_api::cli_state::NodeState;
    use ockam_multiaddr::MultiAddr;

    use super::*;

    pub fn set_space(state: &NodeState, space: &Space) -> Result<()> {
        state.config.lookup.set_space(&space.id, &space.name)?;
        Ok(())
    }

    pub fn set_spaces(state: &NodeState, spaces: &[Space]) -> Result<()> {
        for space in spaces.iter() {
            state.config.lookup.set_space(&space.id, &space.name)?;
        }
        Ok(())
    }

    pub fn remove_space(state: &NodeState, name: &str) -> Result<()> {
        state.config.lookup.remove_space(name)?;
        Ok(())
    }

    pub async fn get_space(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        space_name: &str,
        api_node: &str,
        controller_route: &MultiAddr,
        tcp: Option<&TcpTransport>,
    ) -> Result<String> {
        let state = opts.state.nodes.get(api_node)?;
        match try_get_space(&state, space_name) {
            Some(id) => Ok(id),
            None => {
                refresh_spaces(ctx, opts, api_node, controller_route, tcp).await?;
                let state = opts.state.nodes.get(api_node)?;
                try_get_space(&state, space_name)
                    .context(format!("Space '{}' does not exist", space_name))
            }
        }
    }

    pub fn try_get_space(state: &NodeState, name: &str) -> Option<String> {
        state.config.lookup.get_space(name).map(|s| s.id)
    }

    pub async fn refresh_spaces(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_node: &str,
        controller_route: &MultiAddr,
        tcp: Option<&TcpTransport>,
    ) -> Result<()> {
        if !opts.state.nodes.is_enrolled(api_node)? {
            return Ok(());
        }

        let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp)?.build();
        rpc.request(api::space::list(controller_route)).await?;
        let spaces = rpc.parse_response::<Vec<Space>>()?;
        let state = opts.state.nodes.get(api_node)?;
        set_spaces(&state, &spaces)?;
        Ok(())
    }
}
