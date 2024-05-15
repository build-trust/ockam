use crate::kafka::kafka_default_project_route;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::{command, Args};
use colorful::Colorful;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::services::{StartKafkaInletRequest, StartServiceRequest};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::port_range::PortRange;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::kafka::make_brokers_port_range;
use crate::node::util::initialize_default_node;
use crate::service::start::start_service_impl;
use crate::util::process_nodes_multiaddr;
use crate::{
    kafka::{kafka_default_inlet_bind_address, kafka_inlet_default_addr},
    node::NodeOpts,
    util::parsers::socket_addr_parser,
    Command, CommandGlobalOpts,
};

/// Create a new Kafka Inlet. Kafka clients v3.7.0 and earlier are supported. You can find the version you have with 'kafka-topics.sh --version'.
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,
    /// The local address of the service
    #[arg(long, default_value_t = kafka_inlet_default_addr())]
    pub addr: String,
    /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
    /// In case just a port is specified, the default loopback address (127.0.0.1:4000) will be used
    #[arg(long, default_value_t = kafka_default_inlet_bind_address(), value_parser = socket_addr_parser)]
    pub from: SocketAddr,
    /// Local port range dynamically allocated to kafka brokers, must not overlap with the
    /// bootstrap port
    #[arg(long)]
    pub brokers_port_range: Option<PortRange>,
    /// The route to the Kafka outlet node, either the project in ockam orchestrator or a rust node, expected something like /project/<name>.
    /// Use self when the Kafka outlet is local.
    #[arg(long, default_value_t = kafka_default_project_route())]
    pub to: MultiAddr,
    /// The route to a single Kafka consumer node
    #[arg(long)]
    pub consumer: Option<MultiAddr>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "kafka-inlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let is_finished = Arc::new(Mutex::new(false));
        initialize_default_node(ctx, &opts).await?;

        let brokers_port_range = self
            .brokers_port_range
            .unwrap_or_else(|| make_brokers_port_range(&self.from));
        let at_node = self.node_opts.at_node.clone();
        let addr = self.addr.clone();

        let inlet_creation_future;
        let is_finished = is_finished.clone();
        let to = process_nodes_multiaddr(&self.to, &opts.state).await?;
        {
            let is_finished = is_finished.clone();
            inlet_creation_future = Some(async move {
                let node = BackgroundNodeClient::create(ctx, &opts.state, &at_node).await?;

                let payload = StartKafkaInletRequest::new(self.from, brokers_port_range, to);
                let payload = StartServiceRequest::new(payload, &addr);
                let req = Request::post("/node/services/kafka_inlet").body(payload);
                start_service_impl(ctx, &node, "KafkaInlet", req).await?;

                *is_finished.lock().await = true;
                Ok(())
            });
        }

        opts.terminal
            .write_line(&fmt_log!("Creating KafkaInlet service...\n"))?;

        let msgs = vec![
            format!(
                "Building KafkaInlet service {}",
                &self
                    .addr
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
            format!(
                "Creating KafkaInlet service at {}",
                &self
                    .from
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
            format!(
                "Setting brokers port range to {}",
                &brokers_port_range
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
        ];
        let progress_output = opts.terminal.loop_messages(&msgs, &is_finished);
        let (_, _) = try_join!(inlet_creation_future.unwrap(), progress_output)?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "KafkaInlet service started at {}\n",
                    &self
                        .from
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ) + &fmt_log!(
                    "Brokers port range set to {}\n\n",
                    &brokers_port_range
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ) + &fmt_log!(
                    "{}\n",
                    "Kafka clients v3.7.0 and earlier are supported."
                        .color(OckamColor::FmtWARNBackground.color())
                ) + &fmt_log!(
                    "{}: '{}'.",
                    "You can find the version you have with"
                        .color(OckamColor::FmtWARNBackground.color()),
                    "kafka-topics.sh --version".color(OckamColor::Success.color())
                ),
            )
            .write_line()?;

        Ok(())
    }
}
