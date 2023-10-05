use std::{
    net::{SocketAddr, TcpListener},
    path::Path,
    str::FromStr,
};

use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use tracing::error;

use ockam::{Address, Context, NodeBuilder};
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::config::lookup::{InternetAddress, LookupMeta};
use ockam_core::DenyAll;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Space, Tcp};
use ockam_multiaddr::{
    proto::{self, Node},
    MultiAddr, Protocol,
};

use crate::Result;

pub mod api;
pub mod duration;
pub mod exitcode;
pub mod parsers;

/// A simple wrapper for shutting down the local embedded node (for
/// the client side of the CLI).  Swallows errors and turns them into
/// eprintln logs.
///
/// TODO: We may want to change this behaviour in the future.
pub async fn stop_node(ctx: Context) {
    if let Err(e) = ctx.stop().await {
        eprintln!("an error occurred while shutting down local node: {e}");
    }
}

pub fn local_cmd(res: miette::Result<()>) {
    if let Err(e) = res {
        error!(%e, "Failed to run command");
        eprintln!("{:?}", e);
        std::process::exit(exitcode::SOFTWARE);
    }
}

pub fn node_rpc<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<()>> + Send + 'static,
{
    let res = embedded_node(
        |ctx, a| async {
            let res = f(ctx, a).await;
            if let Err(e) = res {
                error!(%e, "Failed to run command");
                eprintln!("{:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
            Ok(())
        },
        a,
    );
    if let Err(e) = res {
        eprintln!("Ockam runtime failed: {e}");
        std::process::exit(exitcode::SOFTWARE);
    }
}

pub fn embedded_node<A, F, Fut, T>(f: F, a: A) -> miette::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor
        .execute(async move {
            let child_ctx = ctx
                .new_detached(
                    Address::random_tagged("Detached.embedded_node"),
                    DenyAll,
                    DenyAll,
                )
                .await
                .expect("Embedded node child ctx can't be created");
            let r = f(child_ctx, a).await;
            stop_node(ctx).await;
            r
        })
        .into_diagnostic()?
}

pub fn embedded_node_that_is_not_stopped<A, F, Fut, T>(f: F, a: A) -> miette::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor
        .execute(async move {
            let child_ctx = ctx
                .new_detached(
                    Address::random_tagged("Detached.embedded_node.not_stopped"),
                    DenyAll,
                    DenyAll,
                )
                .await
                .expect("Embedded node child ctx can't be created");
            let result = f(child_ctx, a).await;
            if result.is_err() {
                ctx.stop().await.into_diagnostic()?;
                result
            } else {
                result
            }
        })
        .into_diagnostic()?
}

pub fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .into_diagnostic()
        .context("Unable to bind to an open port")?;
    let address = listener
        .local_addr()
        .into_diagnostic()
        .context("Unable to get local address")?;
    Ok(address.port())
}

#[allow(unused)]
pub fn print_path(p: &Path) -> String {
    p.to_str().unwrap_or("<unprintable>").to_string()
}

/// Parses a node's input string for its name in case it's a `MultiAddr` string.
///
/// Ensures that the node's name will be returned if the input string is a `MultiAddr` of the `node` type
/// Examples: `n1` or `/node/n1` returns `n1`; `/project/p1` or `/tcp/n2` returns an error message.
pub fn parse_node_name(input: &str) -> Result<String> {
    if input.is_empty() {
        return Err(miette!("Empty address in node name argument").into());
    }
    // Node name was passed as "n1", for example
    if !input.contains('/') {
        return Ok(input.to_string());
    }
    // Input has "/", so we process it as a MultiAddr
    let maddr = MultiAddr::from_str(input)
        .into_diagnostic()
        .wrap_err("Invalid format for node name argument")?;
    let err_message = String::from("A node MultiAddr must follow the format /node/<name>");
    if let Some(p) = maddr.iter().next() {
        if p.code() == proto::Node::CODE {
            let node_name = p
                .cast::<proto::Node>()
                .ok_or(miette!("Failed to parse the 'node' protocol"))?
                .to_string();
            if !node_name.is_empty() {
                return Ok(node_name);
            }
        }
    }
    Err(miette!(err_message).into())
}

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub fn process_nodes_multiaddr(addr: &MultiAddr, cli_state: &CliState) -> crate::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| miette!("Invalid node address protocol"))?;
                let node_state = cli_state.nodes.get(alias.to_string())?;
                let node_setup = node_state.config().setup();
                let addr = node_setup.api_transport()?.maddr()?;
                processed_addr.try_extend(&addr)?
            }
            _ => processed_addr.push_back_value(&proto)?,
        }
    }
    Ok(processed_addr)
}

/// Go through a multiaddr and remove all instances of
/// `/node/<whatever>` out of it and replaces it with a fully
/// qualified address to the target
pub fn clean_nodes_multiaddr(
    input: &MultiAddr,
    cli_state: &CliState,
) -> Result<(MultiAddr, LookupMeta)> {
    let mut new_ma = MultiAddr::default();
    let mut lookup_meta = LookupMeta::default();
    let it = input.iter().peekable();
    for p in it {
        match p.code() {
            Node::CODE => {
                let alias = p.cast::<Node>().expect("Failed to parse node name");
                let node_state = cli_state.nodes.get(alias.to_string())?;
                let node_setup = node_state.config().setup();
                let addr = &node_setup.api_transport()?.addr;
                match addr {
                    InternetAddress::Dns(dns, _) => new_ma.push_back(DnsAddr::new(dns))?,
                    InternetAddress::V4(v4) => new_ma.push_back(Ip4(*v4.ip()))?,
                    InternetAddress::V6(v6) => new_ma.push_back(Ip6(*v6.ip()))?,
                }
                new_ma.push_back(Tcp(addr.port()))?;
            }
            Project::CODE => {
                // Parse project name from the MultiAddr.
                let alias = p.cast::<Project>().expect("Failed to parse project name");
                // Store it in the lookup meta, so we can later
                // retrieve it from either the config or the cloud.
                lookup_meta.project.push_back(alias.to_string());
                // No substitution done here. It will be done later by `clean_projects_multiaddr`.
                new_ma.push_back_value(&p)?
            }
            Space::CODE => return Err(miette!("/space/ substitutions are not supported!").into()),
            _ => new_ma.push_back_value(&p)?,
        }
    }

    Ok((new_ma, lookup_meta))
}

pub fn comma_separated<T: AsRef<str>>(data: &[T]) -> String {
    use itertools::Itertools;

    #[allow(unstable_name_collisions)]
    data.iter().map(AsRef::as_ref).intersperse(", ").collect()
}

pub fn port_is_free_guard(address: &SocketAddr) -> Result<()> {
    let port = address.port();
    let ip = address.ip();
    if TcpListener::bind((ip, port)).is_err() {
        return Err(miette!("Another process is already listening on port {port}!").into());
    }
    Ok(())
}

pub fn is_tty<S: io_lifetimes::AsFilelike>(s: S) -> bool {
    use is_terminal::IsTerminal;
    s.is_terminal()
}

pub fn is_enrolled_guard(cli_state: &CliState, identity_name: Option<&str>) -> miette::Result<()> {
    if !cli_state
        .identities
        .get_or_default(identity_name)
        .map(|s| s.is_enrolled())
        .unwrap_or(false)
    {
        return Err(miette!(
            "Please enroll using 'ockam enroll' before using this command"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ockam_api::address::extract_address_value;
    use ockam_api::cli_state;
    use ockam_api::cli_state::identities::IdentityConfig;
    use ockam_api::cli_state::traits::StateDirTrait;
    use ockam_api::cli_state::{NodeConfig, VaultConfig};
    use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};

    use super::*;

    #[test]
    fn test_parse_node_name() {
        let test_cases = vec![
            ("", Err(())),
            ("test", Ok("test")),
            ("/test", Err(())),
            ("test/", Err(())),
            ("/node", Err(())),
            ("/node/", Err(())),
            ("/node/n1", Ok("n1")),
            ("/service/s1", Err(())),
            ("/project/p1", Err(())),
            ("/randomprotocol/rp1", Err(())),
            ("/node/n1/tcp", Err(())),
            ("/node/n1/test", Err(())),
            ("/node/n1/tcp/22", Ok("n1")),
        ];
        for (input, expected) in test_cases {
            if let Ok(addr) = expected {
                assert_eq!(parse_node_name(input).unwrap(), addr);
            } else {
                assert!(parse_node_name(input).is_err());
            }
        }
    }

    #[test]
    fn test_extract_address_value() {
        let test_cases = vec![
            ("", Err(())),
            ("test", Ok("test")),
            ("/test", Err(())),
            ("test/", Err(())),
            ("/node", Err(())),
            ("/node/", Err(())),
            ("/node/n1", Ok("n1")),
            ("/service/s1", Ok("s1")),
            ("/project/p1", Ok("p1")),
            ("/randomprotocol/rp1", Err(())),
            ("/node/n1/tcp", Err(())),
            ("/node/n1/test", Err(())),
            ("/node/n1/tcp/22", Ok("n1")),
        ];
        for (input, expected) in test_cases {
            if let Ok(addr) = expected {
                assert_eq!(extract_address_value(input).unwrap(), addr);
            } else {
                assert!(extract_address_value(input).is_err());
            }
        }
    }

    #[ockam_macros::test(crate = "ockam")]
    async fn test_process_multi_addr(ctx: &mut Context) -> ockam::Result<()> {
        let cli_state = CliState::test()?;

        let v_name = cli_state::random_name();
        let v_config = VaultConfig::default();
        cli_state.vaults.create_async(&v_name, v_config).await?;
        let v = cli_state.vaults.get(&v_name)?.get().await?;
        let idt = cli_state
            .get_identities(v)
            .await
            .unwrap()
            .identities_creation()
            .create_identity()
            .await?;
        let idt_config = IdentityConfig::new(idt.identifier()).await;
        cli_state
            .identities
            .create(cli_state::random_name(), idt_config)?;

        let n_state = cli_state
            .nodes
            .create("n1", NodeConfig::try_from(&cli_state)?)?;
        n_state.set_setup(&n_state.config().setup_mut().set_api_transport(
            CreateTransportJson::new(TransportType::Tcp, TransportMode::Listen, "127.0.0.0:4000")?,
        ))?;

        let test_cases = vec![
            (
                MultiAddr::from_str("/node/n1").unwrap(),
                Ok("/ip4/127.0.0.0/tcp/4000"),
            ),
            (
                MultiAddr::from_str("/project/p1").unwrap(),
                Ok("/project/p1"),
            ),
            (
                MultiAddr::from_str("/service/s1").unwrap(),
                Ok("/service/s1"),
            ),
            (
                MultiAddr::from_str("/project/p1/node/n1/service/echo").unwrap(),
                Ok("/project/p1/ip4/127.0.0.0/tcp/4000/service/echo"),
            ),
            (MultiAddr::from_str("/node/n2").unwrap(), Err(())),
        ];
        for (ma, expected) in test_cases {
            if let Ok(addr) = expected {
                let result = process_nodes_multiaddr(&ma, &cli_state)
                    .unwrap()
                    .to_string();
                assert_eq!(result, addr);
            } else {
                assert!(process_nodes_multiaddr(&ma, &cli_state).is_err());
            }
        }

        ctx.stop().await?;
        Ok(())
    }

    #[test]
    fn test_execute_error() {
        let result = embedded_node_that_is_not_stopped(function_returning_an_error, 1);
        assert!(result.is_err());

        async fn function_returning_an_error(_ctx: Context, _parameter: u8) -> miette::Result<()> {
            Err(miette!("boom"))
        }
    }

    #[test]
    fn test_execute_error_() {
        let result = embedded_node_that_is_not_stopped(
            function_returning_an_error_and_stopping_the_context,
            1,
        );
        assert!(result.is_err());

        async fn function_returning_an_error_and_stopping_the_context(
            ctx: Context,
            _parameter: u8,
        ) -> miette::Result<()> {
            ctx.stop().await.into_diagnostic()?;
            Err(miette!("boom"))
        }
    }
}
