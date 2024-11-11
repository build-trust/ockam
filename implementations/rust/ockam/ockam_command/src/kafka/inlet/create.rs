use crate::kafka::kafka_default_project_route;
use crate::kafka::make_brokers_port_range;
use crate::node::util::initialize_default_node;
use crate::tcp::util::alias_parser;
use crate::util::parsers::hostname_parser;
use crate::util::{print_warning_for_deprecated_flag_replaced, process_nodes_multiaddr};
use crate::{
    kafka::{kafka_default_inlet_bind_address, kafka_inlet_default_addr},
    node::NodeOpts,
    Command, CommandGlobalOpts,
};
use async_trait::async_trait;
use clap::{command, Args};
use colorful::Colorful;
use miette::miette;
use ockam::transport::SchemeHostnamePort;
use ockam_abac::PolicyExpression;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::config::lookup::InternetAddress;
use ockam_api::kafka::{ConsumerPublishing, ConsumerResolution};
use ockam_api::nodes::models::services::{StartKafkaInletRequest, StartServiceRequest};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::port_range::PortRange;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use serde::Serialize;
use std::fmt::Write;

/// Create a new Kafka Inlet.
/// Kafka clients v3.7.0 and earlier are supported.
/// You can find the version you have with 'kafka-topics.sh --version'.
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Assign a name to this Kafka Inlet
    #[arg(default_value_t = kafka_inlet_default_addr(), value_parser = alias_parser)]
    pub name: String,

    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// [DEPRECATED] Use the <NAME> positional argument instead
    #[arg(long, default_value_t = kafka_inlet_default_addr())]
    pub addr: String,

    /// The address where the client will connect, in the format `<scheme>://<hostname>:<port>`.
    #[arg(long, id = "SOCKET_ADDRESS", default_value_t = kafka_default_inlet_bind_address(), value_parser = hostname_parser)]
    pub from: SchemeHostnamePort,

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
    #[arg(
        long,
        visible_alias = "avoid-publishing",
        conflicts_with = "publishing-relay"
    )]
    pub no_publishing: bool,

    /// Disable end-to-end kafka messages encryption between producer and consumer.
    /// Use it when you want a plain kafka portal, the communication itself will still be
    /// encrypted.
    #[arg(
        long,
        visible_alias = "disable-content-encryption",
        value_name = "BOOL",
        default_value_t = false
    )]
    pub no_content_encryption: bool,

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
        let cmd = self.parse_args(&opts).await?;

        let inlet = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating Kafka Inlet at {}...\n",
                    color_primary(cmd.from.to_string())
                ));
            }

            let node =
                BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;

            let consumer_resolution;
            if let Some(route) = &cmd.consumer {
                consumer_resolution = ConsumerResolution::SingleNode(route.clone());
            } else if let Some(route) = &cmd.consumer_relay {
                consumer_resolution = ConsumerResolution::ViaRelay(route.clone());
            } else {
                consumer_resolution = ConsumerResolution::ViaRelay(cmd.to.clone());
            }

            let consumer_publishing;
            if cmd.no_publishing {
                consumer_publishing = ConsumerPublishing::None;
            } else if let Some(route) = &cmd.publishing_relay {
                consumer_publishing = ConsumerPublishing::Relay(route.clone());
            } else if let Some(route) = &cmd.consumer_relay {
                consumer_publishing = ConsumerPublishing::Relay(route.clone());
            } else {
                consumer_publishing = ConsumerPublishing::Relay(cmd.to.clone());
            }

            let payload = StartKafkaInletRequest::new(
                cmd.from.clone().into(),
                cmd.brokers_port_range(),
                cmd.to.clone(),
                !cmd.no_content_encryption,
                cmd.encrypted_fields.clone(),
                consumer_resolution,
                consumer_publishing,
                cmd.inlet_policy_expression.clone(),
                cmd.consumer_policy_expression.clone(),
                cmd.producer_policy_expression.clone(),
            );
            let payload = StartServiceRequest::new(payload, &cmd.name);
            let req = Request::post("/node/services/kafka_inlet").body(payload);
            node.tell(ctx, req)
                .await
                .map_err(|e| miette!("Failed to start Kafka Inlet: {e}"))?;

            KafkaInletOutput {
                node_name: node.node_name(),
                from: InternetAddress::new(&cmd.from.hostname_port().to_string())
                    .ok_or(miette!("Invalid address"))?,
                brokers_port_range: cmd.brokers_port_range(),
                to: cmd.to.clone(),
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

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        if self.addr != kafka_inlet_default_addr() {
            print_warning_for_deprecated_flag_replaced(
                opts,
                "addr",
                "the <NAME> positional argument",
            )?;
            if self.name != kafka_inlet_default_addr() {
                opts.terminal.write_line(
                    fmt_warn!("The <NAME> argument is being overridden by the --alias flag")
                        + &fmt_log!("Consider removing the --addr flag"),
                )?;
            }
            self.name = self.addr.clone();
        }

        self.brokers_port_range = self
            .brokers_port_range
            .or_else(|| Some(make_brokers_port_range(&self.from)));

        // The bootstrap port can't overlap with the brokers port range
        if self.from.port() >= self.brokers_port_range().start()
            && self.from.port() <= self.brokers_port_range().end()
        {
            return Err(miette!(
                "The bootstrap port {} can't overlap with the brokers port range {}",
                color_primary(self.from.port()),
                color_primary(self.brokers_port_range().to_string())
            ));
        }

        self.to = process_nodes_multiaddr(&self.to, &opts.state).await?;
        Ok(self)
    }

    fn brokers_port_range(&self) -> PortRange {
        self.brokers_port_range.unwrap()
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
