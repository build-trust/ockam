use async_trait::async_trait;
use clap::Args;

use ockam_api::nodes::models::services::ServiceStatus;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::{Command, CommandGlobalOpts};

/// List Kafka Outlets
#[derive(Args, Clone, Debug)]
#[command()]
pub struct ListCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "kafka-outlet list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let services: Vec<ServiceStatus> = node
            .ask(
                ctx,
                Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_OUTLET)),
            )
            .await?;

        let plain = opts.terminal.build_list(
            &services,
            &format!("No Kafka Outlets found on {}", node.node_name()),
        )?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json_obj(&services)?
            .write_line()?;

        Ok(())
    }
}
