use std::str::FromStr;

use crate::node::NodeOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::Result;
use crate::{fmt_warn, CommandGlobalOpts};
use anyhow::anyhow;
use clap::{Args, Subcommand};
use colorful::Colorful;
use minicbor::Encode;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::services::{
    StartKafkaConsumerRequest, StartKafkaProducerRequest, StartServiceRequest,
};
use ockam_api::port_range::PortRange;
use ockam_api::DefaultAddress;
use ockam_core::api::{Request, RequestBuilder, Status};
use ockam_core::compat::net::{Ipv4Addr, SocketAddr};
use ockam_multiaddr::MultiAddr;

const KAFKA_DEFAULT_PROJECT_ROUTE: &str = "/project/default";
const KAFKA_DEFAULT_CONSUMER_SERVER: &str = "127.0.0.1:4000";
const KAFKA_DEFAULT_CONSUMER_PORT_RANGE: &str = "4001-4100";
const KAFKA_DEFAULT_PRODUCER_SERVER: &str = "127.0.0.1:5000";
const KAFKA_DEFAULT_PRODUCER_PORT_RANGE: &str = "5001-5100";

/// Start a specified service
#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[command(subcommand)]
    pub create_subcommand: StartSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum StartSubCommand {
    Vault {
        #[arg(default_value_t = vault_default_addr())]
        addr: String,
    },
    Identity {
        #[arg(default_value_t = identity_default_addr())]
        addr: String,
    },
    Authenticated {
        #[arg(default_value_t = authenticated_default_addr())]
        addr: String,
    },
    Verifier {
        #[arg(long, default_value_t = verifier_default_addr())]
        addr: String,
    },
    Credentials {
        #[arg(long)]
        identity: String,

        #[arg(long, default_value_t = credentials_default_addr())]
        addr: String,

        #[arg(long)]
        oneway: bool,
    },
    Authenticator {
        #[arg(long, default_value_t = authenticator_default_addr())]
        addr: String,

        #[arg(long)]
        project: String,
    },
    KafkaConsumer {
        /// The local address of the service
        #[arg(long, default_value_t = kafka_consumer_default_addr())]
        addr: String,
        /// The address where to bind and where the client will connect to alongside its port, <address>:<port>.
        /// In case just a port is specified, the default loopback address (127.0.0.1) will be used
        #[arg(long, default_value_t = kafka_default_consumer_server(), value_parser = parse_bootstrap_server)]
        bootstrap_server: SocketAddr,
        /// Local port range dynamically allocated to kafka brokers, must not overlap with the
        /// bootstrap port
        #[arg(long, default_value_t = kafka_default_consumer_port_range())]
        brokers_port_range: PortRange,
        /// The route to the project in ockam orchestrator, expected something like /project/<name>
        #[arg(long, default_value_t = kafka_default_project_route())]
        project_route: MultiAddr,
    },
    KafkaProducer {
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
    },
}

fn vault_default_addr() -> String {
    DefaultAddress::VAULT_SERVICE.to_string()
}

fn identity_default_addr() -> String {
    DefaultAddress::IDENTITY_SERVICE.to_string()
}

fn authenticated_default_addr() -> String {
    DefaultAddress::AUTHENTICATED_SERVICE.to_string()
}

fn verifier_default_addr() -> String {
    DefaultAddress::VERIFIER.to_string()
}

fn credentials_default_addr() -> String {
    DefaultAddress::CREDENTIALS_SERVICE.to_string()
}

fn authenticator_default_addr() -> String {
    DefaultAddress::DIRECT_AUTHENTICATOR.to_string()
}

fn kafka_consumer_default_addr() -> String {
    DefaultAddress::KAFKA_CONSUMER.to_string()
}

fn kafka_producer_default_addr() -> String {
    DefaultAddress::KAFKA_PRODUCER.to_string()
}

fn kafka_default_project_route() -> MultiAddr {
    MultiAddr::from_str(KAFKA_DEFAULT_PROJECT_ROUTE).expect("Failed to parse default project route")
}

fn kafka_default_consumer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_CONSUMER_SERVER)
        .expect("Failed to parse default consumer server")
}

fn kafka_default_consumer_port_range() -> PortRange {
    PortRange::from_str(KAFKA_DEFAULT_CONSUMER_PORT_RANGE)
        .expect("Failed to parse default consumer port range")
}

fn kafka_default_producer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_PRODUCER_SERVER)
        .expect("Failed to parse default producer server")
}

fn kafka_default_producer_port_range() -> PortRange {
    PortRange::from_str(KAFKA_DEFAULT_PRODUCER_PORT_RANGE)
        .expect("Failed to parse default producer port range")
}

impl StartCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: StartCommand,
) -> crate::Result<()> {
    let node_name = &cmd.node_opts.api_node;
    let tcp = TcpTransport::create(ctx).await?;
    match cmd.create_subcommand {
        StartSubCommand::Vault { addr, .. } => {
            start_vault_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Identity { addr, .. } => {
            start_identity_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Authenticated { addr, .. } => {
            let req = api::start_authenticated_service(&addr);
            start_service_impl(
                ctx,
                &opts,
                node_name,
                &addr,
                "Authenticated",
                req,
                Some(&tcp),
            )
            .await?
        }
        StartSubCommand::Verifier { addr, .. } => {
            start_verifier_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Credentials {
            identity,
            addr,
            oneway,
            ..
        } => {
            let req = api::start_credentials_service(&identity, &addr, oneway);
            start_service_impl(ctx, &opts, node_name, &addr, "Credentials", req, Some(&tcp)).await?
        }
        StartSubCommand::Authenticator { addr, project, .. } => {
            start_authenticator_service(ctx, &opts, node_name, &addr, &project, Some(&tcp)).await?
        }
        StartSubCommand::KafkaConsumer {
            addr,
            bootstrap_server,
            brokers_port_range,
            project_route,
        } => {
            let payload =
                StartKafkaConsumerRequest::new(bootstrap_server, brokers_port_range, project_route);
            let payload = StartServiceRequest::new(payload, &addr);
            let req = Request::post("/node/services/kafka_consumer").body(payload);

            opts.terminal.write_line(&fmt_warn!(
                "Starting KafkaConsumer service at {}",
                &bootstrap_server.to_string()
            ))?;
            opts.terminal.write_line(&fmt_warn!(
                "Brokers port range set to {}",
                &brokers_port_range.to_string()
            ))?;
            start_service_impl(
                ctx,
                &opts,
                node_name,
                &addr,
                "KafkaConsumer",
                req,
                Some(&tcp),
            )
            .await?
        }
        StartSubCommand::KafkaProducer {
            addr,
            bootstrap_server,
            brokers_port_range,
            project_route,
        } => {
            let payload =
                StartKafkaProducerRequest::new(bootstrap_server, brokers_port_range, project_route);
            let payload = StartServiceRequest::new(payload, &addr);
            let req = Request::post("/node/services/kafka_producer").body(payload);
            opts.terminal.write_line(&fmt_warn!(
                "Starting KafkaProducer service at {}",
                &bootstrap_server.to_string()
            ))?;
            opts.terminal.write_line(&fmt_warn!(
                "Brokers port range set to {}",
                &brokers_port_range.to_string()
            ))?;
            start_service_impl(
                ctx,
                &opts,
                node_name,
                &addr,
                "KafkaProducer",
                req,
                Some(&tcp),
            )
            .await?
        }
    }

    Ok(())
}

/// Helper function.
async fn start_service_impl<T>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    serv_name: &str,
    req: RequestBuilder<'_, T>,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()>
where
    T: Encode<()>,
{
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp)?.build();
    rpc.request(req).await?;

    let (res, dec) = rpc.check_response()?;
    match res.status() {
        Some(Status::Ok) => {
            println!("{serv_name} service started at address: {serv_addr}");
            Ok(())
        }
        _ => {
            eprintln!("{}", rpc.parse_err_msg(res, dec));
            Err(anyhow!("Failed to start {serv_name} service").into())
        }
    }
}

/// Helper routine for parsing bootstrap server ip and port from user input
/// It can parse a string containing either an `ip:port` pair or just a `port`
/// into a valid SocketAddr instance.
fn parse_bootstrap_server(bootstrap_server: &str) -> Result<SocketAddr> {
    let addr: Vec<&str> = bootstrap_server.split(':').collect();
    match addr.len() {
        // Only the port is available
        1 => {
            let port: u16 = addr[0]
                .parse()
                .map_err(|_| anyhow!("Invalid port number"))?;
            let ip: Ipv4Addr = [127, 0, 0, 1].into();
            Ok(SocketAddr::new(ip.into(), port))
        }
        // Both the ip and port are available
        2 => {
            let port: u16 = addr[1]
                .parse()
                .map_err(|_| anyhow!("Invalid port number"))?;
            let ip = addr[0]
                .parse::<Ipv4Addr>()
                .map_err(|_| anyhow!("Invalid IP address"))?;
            Ok(SocketAddr::new(ip.into(), port))
        }
        _ => Err(anyhow!("Failed to parse bootstrap server").into()),
    }
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_vault_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_vault_service(serv_addr);
    start_service_impl(ctx, opts, node_name, serv_addr, "Vault", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_identity_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_identity_service(serv_addr);
    start_service_impl(ctx, opts, node_name, serv_addr, "Identity", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_verifier_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_verifier_service(serv_addr);
    start_service_impl(ctx, opts, node_name, serv_addr, "Verifier", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
#[allow(clippy::too_many_arguments)]
pub async fn start_authenticator_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    project: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_authenticator_service(serv_addr, project);
    start_service_impl(ctx, opts, node_name, serv_addr, "Authenticator", req, tcp).await
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::service::start::parse_bootstrap_server;

    #[test]
    fn test_parse_bootstrap_server() {
        // Test case 1: only a port is provided
        let input = "9000";
        let result = parse_bootstrap_server(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000),
                bootstrap_server
            );
        }

        // Test case 2: Any 4 octet combination (IPv4) followed by ":" like in "192.168.0.1:9999"
        let input = "192.168.0.1:9999";
        let result = parse_bootstrap_server(input);
        assert!(result.is_ok());
        if let Ok(bootstrap_server) = result {
            assert_eq!(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 9999),
                bootstrap_server
            );
        }

        // Test case 3: Any other format will throw an error
        let invalid_input = "invalid";
        assert!(parse_bootstrap_server(invalid_input).is_err());

        let invalid_input = "192.168.0.1:invalid";
        assert!(parse_bootstrap_server(invalid_input).is_err());

        let invalid_input = "192.168.0.1:9999:extra";
        assert!(parse_bootstrap_server(invalid_input).is_err());
        let invalid_input = "192,166,0.1:9999";
        assert!(parse_bootstrap_server(invalid_input).is_err());
    }
}
