use anyhow::{anyhow, Context as _};
use clap::Args;
use rand::prelude::random;

use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::is_local_node;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_core::api::Request;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::forwarder::HELP_DETAIL;
use crate::util::output::Output;
use crate::util::{extract_address_value, node_rpc, process_multi_addr, RpcBuilder};
use crate::Result;
use crate::{help, CommandGlobalOpts};

/// Create Forwarders
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct CreateCommand {
    /// Name of the forwarder (optional)
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    forwarder_name: String,

    /// Node for which to create the forwarder
    #[arg(long, id = "NODE", display_order = 900)]
    to: String,

    /// Route to the node at which to create the forwarder (optional)
    #[arg(long, id = "ROUTE", display_order = 900)]
    at: MultiAddr,

    /// Authorized identity for secure channel connection (optional)
    #[arg(long, id = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let api_node = extract_address_value(&cmd.to)?;
    let at_rust_node = is_local_node(&cmd.at).context("Argument --at is not valid")?;

    let lookup = opts.config.lookup();
    let ma = process_multi_addr(&cmd.at, &lookup)?;

    let req = {
        let alias = if at_rust_node {
            format!("forward_to_{}", cmd.forwarder_name)
        } else {
            cmd.forwarder_name.clone()
        };
        let body = if cmd.at.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateForwarder::at_project(ma, Some(alias))
        } else {
            CreateForwarder::at_node(ma, Some(alias), at_rust_node, cmd.authorized)
        };
        Request::post("/node/forwarder").body(body)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &api_node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_and_print_response::<ForwarderInfo>()?;

    Ok(())
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{read_to_str, CmdBuilder, NodePool};
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use predicates::prelude::predicate;

    #[test]
    #[ignore]
    fn create_and_send_message_through_it() -> Result<()> {
        let node_1 = NodePool::pull();
        let node_2 = NodePool::pull();
        let forwarder_alias = hex::encode(&rand::random::<[u8; 4]>());
        let output = CmdBuilder::ockam(&format!(
            "forwarder create {} --at /node/{} --to /node/{}",
            &forwarder_alias,
            &node_1.name(),
            &node_2.name(),
        ))?
        .run()?;
        let assert = output.assert().success();
        let forwarder = read_to_str(&assert.get_output().stdout);
        assert_eq!(
            forwarder,
            &format!("/service/forward_to_{}", &forwarder_alias)
        );

        // Send message through forwarder
        let node_3 = NodePool::pull();
        let output = CmdBuilder::ockam(&format!(
            "message send --from /node/{} --to /node/{}/service/forward_to_{}/service/uppercase hello",
            &node_3.name(),
            &node_1.name(),
            &forwarder_alias
        ))?
            .run()?;
        output
            .assert()
            .success()
            .stdout(predicate::str::contains("HELLO"));
        Ok(())
    }
}
