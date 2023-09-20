use std::net::SocketAddr;

use colorful::Colorful;
use tokio::{sync::Mutex, try_join};

use ockam::Context;
use ockam_api::nodes::models::services::{StartKafkaDirectRequest, StartServiceRequest};
use ockam_api::port_range::PortRange;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, NodeOpts};
use crate::service::start::start_service_impl;
use crate::terminal::OckamColor;
use crate::util::{process_nodes_multiaddr, Rpc};
use crate::{display_parse_logs, fmt_log, fmt_ok, CommandGlobalOpts};

pub struct ArgOpts {
    pub endpoint: String,
    pub kafka_entity: String,
    pub node_opts: NodeOpts,
    pub addr: String,
    pub bind_address: SocketAddr,
    pub brokers_port_range: PortRange,
    pub consumer_route: Option<MultiAddr>,
    pub bootstrap_server: SocketAddr,
}

pub async fn start(ctx: Context, (opts, args): (CommandGlobalOpts, ArgOpts)) -> miette::Result<()> {
    let ArgOpts {
        endpoint,
        kafka_entity,
        node_opts,
        addr,
        bind_address,
        brokers_port_range,
        consumer_route,
        bootstrap_server,
    } = args;

    opts.terminal
        .write_line(&fmt_log!("Creating {} service...\n", kafka_entity))?;

    display_parse_logs(&opts);

    let consumer_route = if let Some(consumer_route) = consumer_route {
        Some(process_nodes_multiaddr(&consumer_route, &opts.state)?)
    } else {
        None
    };

    let is_finished = Mutex::new(false);
    let send_req = async {
        let node_name = get_node_name(&opts.state, &node_opts.at_node);
        let mut rpc = Rpc::background(&ctx, &opts.state, &node_name).await?;

        let payload = StartKafkaDirectRequest::new(
            bind_address.to_owned(),
            bootstrap_server,
            brokers_port_range,
            consumer_route,
        );
        let payload = StartServiceRequest::new(payload, &addr);
        let req = Request::post(endpoint).body(payload);
        start_service_impl(&mut rpc, &kafka_entity, req).await?;

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
            &bind_address
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
                &bind_address
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
