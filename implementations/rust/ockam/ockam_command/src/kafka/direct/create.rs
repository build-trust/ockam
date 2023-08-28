use std::net::SocketAddr;

use crate::kafka::direct::rpc::{start, ArgOpts};
use crate::node::initialize_node_if_default;
use crate::{
    kafka::{
        kafka_default_consumer_port_range, kafka_default_consumer_server,
        kafka_default_outlet_server, kafka_direct_default_addr,
    },
    node::NodeOpts,
    util::{node_rpc, parsers::socket_addr_parser},
    CommandGlobalOpts,
};
use clap::{command, Args};
use ockam_api::port_range::PortRange;
use ockam_multiaddr::MultiAddr;

/// Create a new Kafka Direct Consumer
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
    /// The local address of the service
    #[arg(long, default_value_t = kafka_direct_default_addr())]
    addr: String,
    /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
    /// In case just a port is specified, the default loopback address (127.0.0.1) will be used
    #[arg(long, default_value_t = kafka_default_consumer_server(), value_parser = socket_addr_parser)]
    bind_address: SocketAddr,
    /// The address of the kafka bootstrap broke
    #[arg(long, default_value_t = kafka_default_outlet_server())]
    bootstrap_server: SocketAddr,
    /// Local port range dynamically allocated to kafka brokers, must not overlap with the
    /// bootstrap port
    #[arg(long, default_value_t = kafka_default_consumer_port_range())]
    brokers_port_range: PortRange,
    /// The route to another kafka consumer node
    #[arg(long)]
    consumer_route: Option<MultiAddr>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        let arg_opts = ArgOpts {
            endpoint: "/node/services/kafka_direct".to_string(),
            kafka_entity: "KafkaDirect".to_string(),
            node_opts: self.node_opts,
            addr: self.addr,
            bind_address: self.bind_address,
            brokers_port_range: self.brokers_port_range,
            consumer_route: self.consumer_route,
            bootstrap_server: self.bootstrap_server,
        };
        node_rpc(start, (opts, arg_opts));
    }
}
