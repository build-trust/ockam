use crate::node::default_node_name;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_core::api::{Request, RequestBuilder};

/// Delete a forwarder
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name assigned to Relay that will be deleted
    #[arg(display_order = 900, required = true)]
    relay_name: String,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let relay_name = cmd.relay_name.clone();
    let at = cmd
        .at
        .clone()
        .unwrap_or_else(|| default_node_name(&options.state));
    let node = extract_address_value(&at)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;
    rpc.request(make_api_request(cmd)?).await?;

    rpc.is_ok()?;

    options
        .terminal
        .stdout()
        .plain(format!(
            "{}Relay with name {} on Node {} has been deleted.",
            "✔︎".light_green(),
            relay_name,
            node
        ))
        .machine(&relay_name)
        .json(serde_json::json!({ "forwarder": { "name": relay_name, "node": node } }))
        .write_line()?;
    Ok(())
}

/// Construct a request to delete a relay
fn make_api_request<'a>(cmd: DeleteCommand) -> Result<RequestBuilder<'a>> {
    let remote_address = format!("forward_to_{}", cmd.relay_name);
    let request = Request::delete(format!("/node/forwarder/{remote_address}"));
    Ok(request)
}
