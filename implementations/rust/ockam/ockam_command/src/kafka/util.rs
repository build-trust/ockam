use std::net::SocketAddr;

use colorful::Colorful;
use tokio::{sync::Mutex, try_join};

use ockam::Context;
use ockam_api::nodes::models::services::{StartKafkaProducerRequest, StartServiceRequest};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::port_range::PortRange;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::util::initialize_default_node;
use crate::node::NodeOpts;
use crate::service::start::start_service_impl;
use crate::terminal::OckamColor;
use crate::util::process_nodes_multiaddr;
use crate::{display_parse_logs, fmt_log, fmt_ok, CommandGlobalOpts};

pub struct ArgOpts {
    pub endpoint: String,
    pub kafka_entity: String,
    pub node_opts: NodeOpts,
    pub addr: String,
    pub bootstrap_server: SocketAddr,
    pub brokers_port_range: PortRange,
    pub project_route: MultiAddr,
}

/// Return a range of 100 ports after the bootstrap server port
pub(crate) fn make_brokers_port_range(bootstrap_server: &SocketAddr) -> PortRange {
    let boostrap_server_port = bootstrap_server.port();
    // we can unwrap here because we know that range start <= range end
    PortRange::new(boostrap_server_port + 1, boostrap_server_port + 100).unwrap()
}

pub async fn rpc(ctx: Context, (opts, args): (CommandGlobalOpts, ArgOpts)) -> miette::Result<()> {
    initialize_default_node(&ctx, &opts).await?;
    let ArgOpts {
        endpoint,
        kafka_entity,
        node_opts,
        addr,
        bootstrap_server,
        brokers_port_range,
        project_route,
    } = args;

    opts.terminal
        .write_line(&fmt_log!("Creating {} service...\n", kafka_entity))?;

    display_parse_logs(&opts);

    let project_route = process_nodes_multiaddr(&project_route, &opts.state).await?;

    let is_finished = Mutex::new(false);
    let send_req = async {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &node_opts.at_node).await?;

        let payload = StartKafkaProducerRequest::new(
            bootstrap_server.to_owned(),
            brokers_port_range,
            project_route,
        );
        let payload = StartServiceRequest::new(payload, &addr);
        let req = Request::post(endpoint).body(payload);
        start_service_impl(&ctx, &node, &kafka_entity, req).await?;

        *is_finished.lock().await = true;

        Ok::<_, crate::Error>(())
    };

    let msgs = vec![
        format!(
            "Building {} service {}",
            kafka_entity,
            &addr.to_string().color(OckamColor::PrimaryResource.color())
        ),
        format!(
            "Creating {} service at {}",
            kafka_entity,
            &bootstrap_server
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
    let progress_output = opts.terminal.progress_output(&msgs, &is_finished);
    let (_, _) = try_join!(send_req, progress_output)?;

    opts.terminal
        .stdout()
        .plain(
            fmt_ok!(
                "{} service started at {}\n",
                kafka_entity,
                &bootstrap_server
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!(
                "Brokers port range set to {}\n\n",
                &brokers_port_range
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!(
                "{}\n",
                "Kafka clients v3.4.x are supported.".color(OckamColor::FmtWARNBackground.color())
            ) + &fmt_log!(
                "{}\n",
                "You can find the version you have with 'kafka-console-consumer.sh --version'."
                    .color(OckamColor::FmtWARNBackground.color())
            ),
        )
        .write_line()?;

    Ok(())
}
