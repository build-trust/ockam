use std::net::SocketAddr;

use crate::kafka::direct::command::{start, ArgOpts};
use crate::kafka::util::make_brokers_port_range;
use crate::util::async_cmd;
use crate::{
    kafka::{
        kafka_default_consumer_server, kafka_default_outlet_server, kafka_direct_default_addr,
    },
    node::NodeOpts,
    util::parsers::socket_addr_parser,
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
    #[arg(long)]
    brokers_port_range: Option<PortRange>,
    /// The route to another kafka consumer node
    #[arg(long)]
    consumer_route: Option<MultiAddr>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let cmd_name = self.name();
        let args_opts = ArgOpts {
            endpoint: "/node/services/kafka_direct".to_string(),
            kafka_entity: "KafkaDirect".to_string(),
            node_opts: self.node_opts,
            addr: self.addr,
            bind_address: self.bind_address,
            brokers_port_range: self
                .brokers_port_range
                .unwrap_or_else(|| make_brokers_port_range(&self.bootstrap_server)),
            consumer_route: self.consumer_route,
            bootstrap_server: self.bootstrap_server,
        };
        async_cmd(&cmd_name, opts.clone(), |ctx| async move {
            start(&ctx, opts, args_opts).await
        })
    }

    pub fn name(&self) -> String {
        "create kafka direct".into()
    }
}
