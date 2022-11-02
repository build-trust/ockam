use crate::node::NodeOpts;
use crate::util::output::Output;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use core::fmt::Write;
use ockam::Context;
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};
use ockam_core::api::Request;
use ockam_identity::change_history::IdentityChangeHistory;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
    #[arg(short, long)]
    full: bool,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let node_name = extract_address_value(&cmd.node_opts.api_node)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    if cmd.full {
        let req = Request::post("/node/identity/actions/show/long");
        rpc.request(req).await?;
        rpc.parse_and_print_response::<LongIdentityResponse>()?;
    } else {
        let req = Request::post("/node/identity/actions/show/short");
        rpc.request(req).await?;
        rpc.parse_and_print_response::<ShortIdentityResponse>()?;
    }
    Ok(())
}

impl Output for LongIdentityResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        let id: IdentityChangeHistory = serde_bare::from_slice(self.identity.0.as_ref())?;
        write!(w, "{}", id)?;
        Ok(w)
    }
}

impl Output for ShortIdentityResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "{}", self.identity_id)?;
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{CmdBuilder, NodePool};
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use predicates::prelude::predicate;

    #[test]
    fn show() -> Result<()> {
        let node = NodePool::pull();
        let output = CmdBuilder::ockam(&format!("identity show -n {}", &node.name()))?.run()?;
        output
            .assert()
            .success()
            .stdout(predicate::str::starts_with("P"));
        Ok(())
    }

    #[test]
    fn show_full() -> Result<()> {
        let node = NodePool::pull();
        let output =
            CmdBuilder::ockam(&format!("identity show --full -n {}", &node.name()))?.run()?;
        output
            .assert()
            .success()
            .stdout(predicate::str::contains("Change History"))
            .stdout(predicate::str::contains("identifier"))
            .stdout(predicate::str::contains("signatures"));
        Ok(())
    }
}
