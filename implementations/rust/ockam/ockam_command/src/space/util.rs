use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::Result;

pub mod config {
    use crate::util::{api, RpcBuilder};
    use crate::{CommandGlobalOpts, OckamConfig};
    use ockam_multiaddr::MultiAddr;

    use super::*;

    pub fn set_space(config: &OckamConfig, space: &Space) -> Result<()> {
        config.set_space_alias(&space.id, &space.name);
        config.persist_config_updates()?;
        Ok(())
    }

    pub fn set_spaces(config: &OckamConfig, spaces: &[Space]) -> Result<()> {
        config.remove_spaces_alias();
        for space in spaces.iter() {
            config.set_space_alias(&space.id, &space.name);
        }
        config.persist_config_updates()?;
        Ok(())
    }

    pub fn remove_space(config: &OckamConfig, name: &str) -> Result<()> {
        config.remove_space_alias(name);
        config.persist_config_updates()?;
        Ok(())
    }

    pub async fn get_space(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        space_name: &str,
        api_node: &str,
        controller_route: &MultiAddr,
    ) -> Result<String> {
        match try_get_space(&opts.config, space_name) {
            Ok(id) => Ok(id),
            Err(_) => {
                refresh_spaces(ctx, opts, api_node, controller_route).await?;
                Ok(try_get_space(&opts.config, space_name)?)
            }
        }
    }

    pub fn try_get_space(config: &OckamConfig, name: &str) -> Result<String> {
        let inner = config.write();
        match inner.lookup.get_space(name) {
            Some(s) => Ok(s.id.clone()),
            None => Err(anyhow::anyhow!("Space '{name}' does not exist").into()),
        }
    }

    async fn refresh_spaces(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_node: &str,
        controller_route: &MultiAddr,
    ) -> Result<()> {
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();
        rpc.request(api::space::list(controller_route)).await?;
        let spaces = rpc.parse_response::<Vec<Space>>()?;
        set_spaces(&opts.config, &spaces)?;
        Ok(())
    }
}
