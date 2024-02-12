use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_abac::{Action, Expr, ResourceName, ResourceType};
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};

use super::resource_type_parser;
use crate::node::util::initialize_default_node;
use crate::terminal::color_primary;
use crate::util::async_cmd;
use crate::{fmt_ok, fmt_warn, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    pub at: Option<String>,

    #[arg(
        long = "resource-type",
        conflicts_with = "resource",
        value_parser = resource_type_parser
    )]
    pub resource_type: Option<ResourceType>,

    #[arg(long)]
    pub resource: Option<ResourceName>,

    #[arg(long)]
    pub expression: Expr,
}

impl CreateCommand {
    pub fn run(mut self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create policy".into()
    }

    pub async fn async_run(
        &mut self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        // Backwards compatibility
        if let Some(resource) = self.resource.take() {
            match resource.as_str() {
                "tcp-inlet" => {
                    opts.terminal.write_line(fmt_warn!(
                        "{} is deprecated. Please use {} instead",
                        color_primary("--resource tcp-inlet"),
                        color_primary("--resource-type tcp-inlet")
                    ))?;
                    self.resource_type = Some(ResourceType::TcpInlet);
                }
                "tcp-outlet" => {
                    opts.terminal.write_line(fmt_warn!(
                        "{} is deprecated. Please use {} instead",
                        color_primary("--resource tcp-outlet"),
                        color_primary("--resource-type tcp-outlet")
                    ))?;
                    self.resource_type = Some(ResourceType::TcpOutlet);
                }
                _ => self.resource = Some(resource),
            }
        }

        let resource = ResourceTypeOrName::new(self.resource_type.as_ref(), self.resource.as_ref())
            .into_diagnostic()?;

        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        node.add_policy(ctx, &resource, &Action::HandleMessage, &self.expression)
            .await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Policy created at node {}",
                color_primary(node.node_name())
            ))
            .write_line()?;
        Ok(())
    }
}
