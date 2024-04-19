use crate::kafka::kafka_default_project_route;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::{command, Args};
use colorful::Colorful;
use ockam_abac::PolicyExpression;
use ockam_api::colors::OckamColor;
use ockam_api::kafka::{ConsumerPublishing, ConsumerResolution};
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
    #[arg(long, default_value_t = kafka_default_project_route(), value_name = "ROUTE")]
    pub to: MultiAddr,
    /// The direct route to a single Kafka consumer node instead of using a relay for their
    /// resolution. A single encryption key will be exchanged with the provided consumer.
    #[arg(long, conflicts_with = "consumer-relay", value_name = "ROUTE")]
    pub consumer: Option<MultiAddr>,
    /// The route to the Kafka consumer relay node.
    /// Encryption keys will be exchanged passing through this relay based on topic
    /// and partition name.
    /// By default, this parameter uses the value of `to`.
    #[arg(long, name = "consumer-relay", value_name = "ROUTE")]
    pub consumer_relay: Option<MultiAddr>,
    /// The route to the Kafka consumer relay node which will be used to make this consumer
    /// available to producers.
    /// By default, this parameter uses the value of `consumer-relay`.
    #[arg(long, name = "publishing-relay", value_name = "ROUTE")]
    pub publishing_relay: Option<MultiAddr>,
    /// Avoid publishing the consumer in the relay.
    /// This is useful to avoid the creation of an unused relay when the consumer is directly
    /// referenced by the producer.
    #[arg(long, name = "avoid-publishing", conflicts_with = "publishing-relay")]
    pub avoid_publishing: bool,
    /// Policy expression that will be used for access control to the Kafka Inlet.
    /// If you don't provide it, the policy set for the "tcp-inlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-inlet`.
    #[arg(hide = true, long = "allow", id = "INLET-EXPRESSION")]
    pub inlet_policy_expression: Option<PolicyExpression>,
    /// Policy expression that will be used for access control to the Kafka Consumer.
    /// If you don't provide it, the policy set for the "kafka-consumer" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type kafka-consumer`.
    #[arg(hide = true, long = "allow-consumer", id = "CONSUMER-EXPRESSION")]
    pub consumer_policy_expression: Option<PolicyExpression>,
    /// Policy expression that will be used for access control to the Kafka Producer.
    /// If you don't provide it, the policy set for the "kafka-producer" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type kafka-producer`.
    #[arg(hide = true, long = "allow-producer", id = "PRODUCER-EXPRESSION")]
    pub producer_policy_expression: Option<PolicyExpression>,
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

                let consumer_resolution;
                if let Some(route) = self.consumer {
                    consumer_resolution = ConsumerResolution::SingleNode(route);
                } else if let Some(route) = &self.consumer_relay {
                    consumer_resolution = ConsumerResolution::ViaRelay(route.clone());
                } else {
                    consumer_resolution = ConsumerResolution::ViaRelay(to.clone());
                }

                let consumer_publishing;
                if self.avoid_publishing {
                    consumer_publishing = ConsumerPublishing::None;
                } else if let Some(route) = self.publishing_relay {
                    consumer_publishing = ConsumerPublishing::Relay(route);
                } else if let Some(route) = self.consumer_relay {
                    consumer_publishing = ConsumerPublishing::Relay(route);
                } else {
                    consumer_publishing = ConsumerPublishing::Relay(to.clone());
                }

                let payload = StartKafkaInletRequest::new(
                    self.from,
                    brokers_port_range,
                    to,
                    consumer_resolution,
                    consumer_publishing,
                    self.inlet_policy_expression,
                    self.consumer_policy_expression,
                    self.producer_policy_expression,
                );
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
