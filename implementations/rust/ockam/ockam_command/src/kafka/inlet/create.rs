use crate::kafka::kafka_default_project_route;
use async_trait::async_trait;
use std::fmt::Write;

use clap::{command, Args};
use colorful::Colorful;
use miette::miette;
use ockam::transport::HostnamePort;
use ockam_abac::PolicyExpression;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::config::lookup::InternetAddress;
use ockam_api::kafka::{ConsumerPublishing, ConsumerResolution};
use ockam_api::nodes::models::services::{StartKafkaInletRequest, StartServiceRequest};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::port_range::PortRange;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use serde::Serialize;

use crate::kafka::make_brokers_port_range;
use crate::node::util::initialize_default_node;
use crate::util::process_nodes_multiaddr;
use crate::{
    kafka::{kafka_default_inlet_bind_address, kafka_inlet_default_addr},
    node::NodeOpts,
    util::parsers::hostname_parser,
    Command, CommandGlobalOpts,
};

/// Create a new Kafka Inlet.
/// Kafka clients v3.7.0 and earlier are supported.
/// You can find the version you have with 'kafka-topics.sh --version'.
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// The local address of the service
    #[arg(long, default_value_t = kafka_inlet_default_addr())]
    pub addr: String,

    /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
    /// In case just a port is specified, the default loopback address (127.0.0.1:4000) will be used
    #[arg(long, default_value_t = kafka_default_inlet_bind_address(), value_parser = hostname_parser)]
    pub from: HostnamePort,

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

    /// Disable end-to-end kafka messages encryption between producer and consumer.
    /// Use it when you want a plain kafka portal, the communication itself will still be
    /// encrypted.
    #[arg(
        long,
        name = "disable-content-encryption",
        value_name = "BOOL",
        default_value_t = false
    )]
    pub disable_content_encryption: bool,

    /// The fields to encrypt in the kafka messages, assuming the record is a valid JSON map.
    /// By default, the whole record is encrypted.
    #[arg(
        long,
        alias = "encrypted-fields",
        long = "encrypted-field",
        value_name = "FIELD"
    )]
    pub encrypted_fields: Vec<String>,

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
        initialize_default_node(ctx, &opts).await?;

        let brokers_port_range = self
            .brokers_port_range
            .unwrap_or_else(|| make_brokers_port_range(&self.from));

        // The bootstrap port can't overlap with the brokers port range
        if self.from.port() >= brokers_port_range.start()
            && self.from.port() <= brokers_port_range.end()
        {
            return Err(miette!(
                "The bootstrap port {} can't overlap with the brokers port range {}",
                self.from.port(),
                brokers_port_range.to_string()
            ));
        }

        let at_node = self.node_opts.at_node.clone();
        let addr = self.addr.clone();
        let to = process_nodes_multiaddr(&self.to, &opts.state).await?;

        let inlet = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating Kafka Inlet at {}...\n",
                    color_primary(self.from.to_string())
                ));
            }

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
                self.from.clone(),
                brokers_port_range,
                to.clone(),
                !self.disable_content_encryption,
                self.encrypted_fields,
                consumer_resolution,
                consumer_publishing,
                self.inlet_policy_expression,
                self.consumer_policy_expression,
                self.producer_policy_expression,
            );
            let payload = StartServiceRequest::new(payload, &addr);
            let req = Request::post("/node/services/kafka_inlet").body(payload);
            node.tell(ctx, req)
                .await
                .map_err(|e| miette!("Failed to start Kafka Inlet: {e}"))?;

            KafkaInletOutput {
                node_name: node.node_name(),
                from: InternetAddress::new(&self.from.to_string())
                    .ok_or(miette!("Invalid address"))?,
                brokers_port_range,
                to,
            }
        };

        opts.terminal
            .stdout()
            .plain(inlet.item()?)
            .json_obj(inlet)?
            .write_line()?;

        Ok(())
    }
}

#[derive(Serialize)]
struct KafkaInletOutput {
    node_name: String,
    from: InternetAddress,
    brokers_port_range: PortRange,
    to: MultiAddr,
}

impl Output for KafkaInletOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let mut f = String::new();
        writeln!(
            f,
            "{}\n{}\n{}\n",
            fmt_ok!(
                "Created a new Kafka Inlet in the Node {} bound to {}",
                color_primary(&self.node_name),
                color_primary(self.from.to_string())
            ),
            fmt_log!(
                "with the brokers port range set to {}",
                color_primary(self.brokers_port_range.to_string())
            ),
            fmt_log!(
                "sending traffic to the Kafka Outlet at {}",
                color_primary(self.to.to_string())
            )
        )?;

        writeln!(
            f,
            "{}\n{}",
            fmt_log!(
                "{}",
                color_warn("Kafka clients v3.7.0 and earlier are supported")
            ),
            fmt_log!(
                "{}: {}",
                color_warn("You can find the version you have with"),
                color_primary("kafka-topics.sh --version")
            )
        )?;

        Ok(f)
    }
}
