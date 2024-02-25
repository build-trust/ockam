use std::fmt::Write;

use clap::Args;
use miette::IntoDiagnostic;

use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_abac::{ResourcePolicy, ResourceTypePolicy};
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};

use crate::output::Output;
use crate::terminal::color_primary;
use crate::util::async_cmd;
use crate::{CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    resource: Option<ResourceTypeOrName>,

    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "list policies".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
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

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (policies, _) = try_join!(get_policies, progress_output)?;

        if policies.resource_type_policies().is_empty() && policies.resource_policies().is_empty() {
            let list = opts.terminal.build_list(
                policies.resource_type_policies(),
                "",
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
                    &format!("Resource type policies on Node {}", &node.node_name()),
                    &format!("No resource type policies on Node {}", &node.node_name()),
                )?;
            }
            if !policies.resource_policies().is_empty() {
                plain.push_str(&opts.terminal.build_list(
                    policies.resource_policies(),
                    &format!("Resource policies on Node {}", &node.node_name()),
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

impl Output for ResourceTypePolicy {
    fn output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Resource type: {}",
            color_primary(&self.resource_type)
        )?;
        write!(
            output,
            "Expression: {}",
            color_primary(self.expression.to_string())
        )?;
        Ok(output)
    }
}

impl Output for ResourcePolicy {
    fn output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Resource name: {}",
            color_primary(self.resource_name.to_string())
        )?;
        write!(
            output,
            "Expression: {}",
            color_primary(self.expression.to_string())
        )?;
        Ok(output)
    }
}
