use std::net::SocketAddr;

use colorful::Colorful;
use tokio::{sync::Mutex, try_join};

use ockam::Context;
use ockam_api::nodes::models::services::{StartKafkaProducerRequest, StartServiceRequest};
use ockam_api::nodes::RemoteNode;
use ockam_api::port_range::PortRange;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, NodeOpts};
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

pub async fn rpc(ctx: Context, (opts, args): (CommandGlobalOpts, ArgOpts)) -> miette::Result<()> {
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

    let project_route = process_nodes_multiaddr(&project_route, &opts.state)?;

    let is_finished = Mutex::new(false);
    let send_req = async {
        let node_name = get_node_name(&opts.state, &node_opts.at_node);
        let node = RemoteNode::create(&ctx, &opts.state, &node_name).await?;

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
                "Brokers port range set to {}",
                &brokers_port_range
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
        )
        .write_line()?;

    Ok(())
}
