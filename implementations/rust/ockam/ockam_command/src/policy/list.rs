use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;

use ockam_api::colors::color_primary;
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};

use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    resource: Option<ResourceTypeOrName>,

    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "policy list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let output_messages = if self.resource.is_none() {
            vec![format!(
                "Listing Policies on {} for all Resources...\n",
                color_primary(node.node_name())
            )]
        } else {
            vec![format!(
                "Listing Policies on {} for Resource {}...\n",
                color_primary(node.node_name()),
                color_primary(self.resource.as_ref().unwrap().to_string())
            )]
        };

        let get_policies = async {
            let policies = node.list_policies(ctx, self.resource.as_ref()).await?;
            *is_finished.lock().await = true;
            Ok(policies)
        };

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (policies, _) = try_join!(get_policies, progress_output)?;

        if policies.resource_type_policies().is_empty() && policies.resource_policies().is_empty() {
            let list = opts.terminal.build_list(
                policies.resource_type_policies(),
                &format!("No policies on Node {}", &node.node_name()),
            )?;
            opts.terminal.stdout().plain(list).write_line()?;
            return Ok(());
        }

        let json = serde_json::to_string(&policies.all()).into_diagnostic()?;
        let plain = {
            let mut plain = String::new();
            if !policies.resource_type_policies().is_empty() {
                plain = opts.terminal.build_list(
                    policies.resource_type_policies(),
                    &format!("No resource type policies on Node {}", &node.node_name()),
                )?;
            }
            if !policies.resource_policies().is_empty() {
                plain.push_str(&opts.terminal.build_list(
                    policies.resource_policies(),
                    &format!("No resource policies on Node {}", &node.node_name()),
                )?);
            }
            plain
        };
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
