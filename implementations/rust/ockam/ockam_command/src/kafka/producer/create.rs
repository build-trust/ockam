use std::net::SocketAddr;

use clap::{command, Args};
use colorful::Colorful;
use ockam::{Context, TcpTransport};
use ockam_api::{
    nodes::models::services::{StartKafkaProducerRequest, StartServiceRequest},
    port_range::PortRange,
};
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    fmt_info,
    kafka::{
        kafka_default_producer_port_range, kafka_default_producer_server,
        kafka_default_project_route, kafka_producer_default_addr, parse_bootstrap_server,
    },
    node::NodeOpts,
    service::start::start_service_impl,
    util::node_rpc,
    CommandGlobalOpts,
};

/// Create a new Kafka Producer
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// The local address of the service
    #[arg(long, default_value_t = kafka_producer_default_addr())]
    addr: String,
    /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
    /// In case just a port is specified, the default loopback address (127.0.0.1) will be used
    #[arg(long, default_value_t = kafka_default_producer_server(), value_parser = parse_bootstrap_server)]
    bootstrap_server: SocketAddr,
    /// Local port range dynamically allocated to kafka brokers, must not overlap with the
    /// bootstrap port
    #[arg(long, default_value_t = kafka_default_producer_port_range())]
    brokers_port_range: PortRange,
    /// The route to the project in ockam orchestrator, expected something like /project/<name>
    #[arg(long, default_value_t = kafka_default_project_route())]
    project_route: MultiAddr,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> crate::Result<()> {
    let CreateCommand {
        node_opts,
        addr,
        bootstrap_server,
        brokers_port_range,
        project_route,
    } = cmd;
    let tcp = TcpTransport::create(&ctx).await?;

    let payload =
        StartKafkaProducerRequest::new(bootstrap_server, brokers_port_range, project_route);
    let payload = StartServiceRequest::new(payload, &addr);
    let req = Request::post("/node/services/kafka_producer").body(payload);
    opts.terminal.write_line(&fmt_info!(
        "Starting KafkaProducer service at {}",
        &bootstrap_server.to_string()
    ))?;
    opts.terminal.write_line(&fmt_info!(
        "Brokers port range set to {}",
        &brokers_port_range.to_string()
    ))?;

    start_service_impl(
        &ctx,
        &opts,
        &node_opts.api_node,
        &addr,
        "KafkaProducer",
        req,
        Some(&tcp),
    )
    .await?;

    Ok(())
}
