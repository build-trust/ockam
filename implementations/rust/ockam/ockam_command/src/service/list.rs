use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::services::ServiceStatus;
use ockam_api::nodes::BackgroundNodeClient;

use crate::node::NodeOpts;
use crate::util::api;
use crate::CommandGlobalOpts;

/// List service(s) of a given node
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "service list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_services = async {
            let services: Vec<ServiceStatus> = node.ask(ctx, api::list_services()).await?;
            *is_finished.lock().await = true;
            Ok(services)
        };

        let output_messages = vec![format!(
            "Listing Services on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (services, _) = try_join!(get_services, progress_output)?;

        let plain = opts.terminal.build_list(
            &services,
            &format!("No services found on {}", node.node_name()),
        )?;
        let json = serde_json::to_string(&services).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
