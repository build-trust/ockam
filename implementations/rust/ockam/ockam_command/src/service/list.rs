use std::fmt::Write;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::nodes::models::services::{ServiceList, ServiceStatus};
use ockam_api::nodes::BackgroundNodeClient;

use crate::node::NodeOpts;
use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{api, async_cmd};
use crate::CommandGlobalOpts;

/// List service(s) of a given node
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "service list".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_services = async {
            let services: ServiceList = node.ask(ctx, api::list_services()).await?;
            *is_finished.lock().await = true;
            Ok(services)
        };

        let output_messages = vec![format!(
            "Listing Services on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (services, _) = try_join!(get_services, progress_output)?;

        let plain = opts.terminal.build_list(
            &services.list,
            &format!("Services on {}", node.node_name()),
            &format!("No services found on {}", node.node_name()),
        )?;
        let json = serde_json::to_string_pretty(&services.list).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}

impl Output for ServiceStatus {
    fn output(&self) -> crate::Result<String> {
        let mut output = String::new();

        writeln!(
            output,
            "Service {}",
            self.service_type
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "Address {}{}",
            "/service/"
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.addr
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;

        Ok(output)
    }
}
