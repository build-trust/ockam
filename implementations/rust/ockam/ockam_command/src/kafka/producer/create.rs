use std::net::SocketAddr;

use clap::{command, Args};

use ockam_api::port_range::PortRange;
use ockam_multiaddr::MultiAddr;

use crate::util::print_deprecated_warning;
use crate::{
    kafka::{kafka_default_producer_server, kafka_default_project_route, kafka_inlet_default_addr},
    node::NodeOpts,
    util::parsers::socket_addr_parser,
    Command, CommandGlobalOpts,
};

/// Create a new Kafka Producer. Kafka clients v3.7.0 and earlier are supported. You can find the version you have with 'kafka-console-producer.sh --version'. [DEPRECATED]
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
    /// The local address of the service
    #[arg(long, default_value_t = kafka_inlet_default_addr())]
    addr: String,
    /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
    /// In case just a port is specified, the default loopback address (127.0.0.1) will be used
    #[arg(long, default_value_t = kafka_default_producer_server(), value_parser = socket_addr_parser)]
    bootstrap_server: SocketAddr,
    /// Local port range dynamically allocated to kafka brokers, must not overlap with the
    /// bootstrap port
    #[arg(long)]
    brokers_port_range: Option<PortRange>,
    /// The route to the project in ockam orchestrator, expected something like /project/<name>
    #[arg(long, default_value_t = kafka_default_project_route())]
    project_route: MultiAddr,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        print_deprecated_warning(&opts, &self.name(), "kafka-inlet")?;
        crate::kafka::inlet::create::CreateCommand {
            node_opts: self.node_opts,
            addr: self.addr,
            from: self.bootstrap_server,
            brokers_port_range: self.brokers_port_range,
            to: self.project_route,
            consumer: None,
            consumer_relay: None,
            publishing_relay: None,
            avoid_publishing: false,
            disable_content_encryption: false,
            inlet_policy_expression: None,
            consumer_policy_expression: None,
            producer_policy_expression: None,
        }
        .run(opts)
    }

    pub fn name(&self) -> String {
        "kafka-producer create".into()
    }
}
