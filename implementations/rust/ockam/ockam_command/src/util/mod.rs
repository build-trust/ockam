use std::{
    net::{SocketAddr, TcpListener},
    path::Path,
};

use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use tracing::error;

use ockam::{Address, Context, NodeBuilder};
use ockam_api::cli_state::CliState;
use ockam_api::config::lookup::{InternetAddress, LookupMeta};
use ockam_core::DenyAll;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Space, Tcp};
use ockam_multiaddr::{proto::Node, MultiAddr, Protocol};

use crate::error::Error;
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
    let res = executor.execute(async move {
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
        r.map_err(|e| {
            ockam_core::Error::new(
                ockam_core::errcode::Origin::Executor,
                ockam_core::errcode::Kind::Unknown,
                e,
            )
        })
    });
    let res = res.map_err(|e| miette::miette!(e));
    res?.into_diagnostic()
}

pub fn embedded_node_that_is_not_stopped<A, F, Fut, T>(f: F, a: A) -> miette::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    let res = executor.execute(async move {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Detached.embedded_node.not_stopped"),
                DenyAll,
                DenyAll,
            )
            .await
            .expect("Embedded node child ctx can't be created");
        let result = f(child_ctx, a).await;
        let result = if result.is_err() {
            ctx.stop().await?;
            result
        } else {
            result
        };
        result.map_err(|e| {
            ockam_core::Error::new(
                ockam_core::errcode::Origin::Executor,
                ockam_core::errcode::Kind::Unknown,
                e,
            )
        })
    });

    let res = res.map_err(|e| miette::miette!(e));
    res?.into_diagnostic()
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

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub async fn process_nodes_multiaddr(
    addr: &MultiAddr,
    cli_state: &CliState,
) -> crate::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| miette!("Invalid node address protocol"))?;
                let node_info = cli_state.get_node(&alias).await?;
                let addr = node_info.tcp_listener_multi_address()?;
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
pub async fn clean_nodes_multiaddr(
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
                let node_info = cli_state.get_node(&alias).await?;
                let addr = node_info
                    .tcp_listener_address()
                    .ok_or(Error::new_internal_error(
                        "No transport API has been set on the node",
                        "",
                    ))?;
                match &addr {
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
            Space::CODE => return Err(miette!("/space/ substitutions are not supported!"))?,
            _ => new_ma.push_back_value(&p)?,
        }
    }

    Ok((new_ma, lookup_meta))
}

pub fn comma_separated<T: AsRef<str>>(data: &[T]) -> String {
    data.iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn port_is_free_guard(address: &SocketAddr) -> Result<()> {
    let port = address.port();
    let ip = address.ip();
    if TcpListener::bind((ip, port)).is_err() {
        return Err(miette!(
            "Another process is already listening on port {port}!"
        ))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[ockam_macros::test(crate = "ockam")]
    async fn test_process_multi_addr(ctx: &mut Context) -> ockam::Result<()> {
        let cli_state = CliState::test().await?;

        cli_state.create_node("n1").await?;

        cli_state
            .set_tcp_listener_address(
                "n1",
                &SocketAddr::from_str("127.0.0.0:4000").unwrap().into(),
            )
            .await?;

        let test_cases = vec![
            (
                MultiAddr::from_str("/node/n1")?,
                Ok("/ip4/127.0.0.0/tcp/4000"),
            ),
            (MultiAddr::from_str("/project/p1")?, Ok("/project/p1")),
            (MultiAddr::from_str("/service/s1")?, Ok("/service/s1")),
            (
                MultiAddr::from_str("/project/p1/node/n1/service/echo")?,
                Ok("/project/p1/ip4/127.0.0.0/tcp/4000/service/echo"),
            ),
            (MultiAddr::from_str("/node/n2")?, Err(())),
        ];
        for (ma, expected) in test_cases {
            if let Ok(addr) = expected {
                let result = process_nodes_multiaddr(&ma, &cli_state)
                    .await
                    .unwrap()
                    .to_string();
                assert_eq!(result, addr);
            } else {
                assert!(process_nodes_multiaddr(&ma, &cli_state).await.is_err());
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

    #[test]
    fn test_comma_separated() {
        let data = vec!["a", "b", "c"];
        let result = comma_separated(&data);
        assert_eq!(result, "a, b, c");
    }
}
