use clap::{command, Args};

use crate::util::print_warning_for_deprecated_flag_replaced;
use crate::{
    kafka::{kafka_default_consumer_server, kafka_default_project_route, kafka_inlet_default_addr},
    node::NodeOpts,
    util::parsers::hostname_parser,
    Command, CommandGlobalOpts,
};

use ockam::transport::SchemeHostnamePort;
use ockam_api::port_range::PortRange;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

/// Create a new Kafka Consumer. Kafka clients v3.7.0 and earlier are supported.
/// You can find the version you have with 'kafka-console-consumer.sh --version'.
/// [DEPRECATED]
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// The local address of the service
    #[arg(long, default_value_t = kafka_inlet_default_addr())]
    addr: String,

    /// The address where the client will connect, in the format `<scheme>://<hostname>:<port>`.
    #[arg(long, id = "SOCKET_ADDRESS", default_value_t = kafka_default_consumer_server(), value_parser = hostname_parser)]
    bootstrap_server: SchemeHostnamePort,

    /// Local port range dynamically allocated to kafka brokers, must not overlap with the
    /// bootstrap port
    #[arg(long)]
    brokers_port_range: Option<PortRange>,

    /// The route to the project in ockam orchestrator, expected something like /project/<name>
    #[arg(long, default_value_t = kafka_default_project_route())]
    project_route: MultiAddr,
}

impl CreateCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        print_warning_for_deprecated_flag_replaced(&opts, &self.name(), "kafka-inlet")?;
        crate::kafka::inlet::create::CreateCommand {
            name: self.addr.clone(),
            node_opts: self.node_opts,
            addr: self.addr,
            from: self.bootstrap_server,
            brokers_port_range: self.brokers_port_range,
            to: self.project_route,
            consumer: None,
            consumer_relay: None,
            publishing_relay: None,
            no_publishing: false,
            no_content_encryption: false,
            encrypted_fields: vec![],
            inlet_policy_expression: None,
            consumer_policy_expression: None,
            producer_policy_expression: None,
        }
        .run(ctx, opts)
        .await
    }

    pub fn name(&self) -> String {
        "kafka-consumer create".into()
    }
}
