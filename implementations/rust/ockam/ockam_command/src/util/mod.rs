use std::sync::Arc;
use std::{
    net::{SocketAddr, TcpListener},
    path::Path,
};

use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use opentelemetry::trace::FutureExt;
use tokio::runtime::Runtime;
use tracing::{debug, error};

use ockam::{Address, Context, NodeBuilder};
use ockam_api::cli_state::CliState;
use ockam_api::cli_state::CliStateError;
use ockam_api::colors::color_primary;
use ockam_api::config::lookup::{InternetAddress, LookupMeta};
use ockam_api::fmt_warn;
use ockam_core::{DenyAll, OpenTelemetryContext};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Space, Tcp};
use ockam_multiaddr::{proto::Node, MultiAddr, Protocol};

use crate::{CommandGlobalOpts, Result};

pub mod api;
pub mod exitcode;
pub mod foreground_args;
pub mod parsers;
pub mod validators;

pub fn local_cmd(res: miette::Result<()>) -> miette::Result<()> {
    if let Err(error) = &res {
        // Note: error! is also called in command_event.rs::add_command_error_event()
        error!(%error, "Failed to run command");
    }
    res
}

pub fn async_cmd<F, Fut>(command_name: &str, opts: CommandGlobalOpts, f: F) -> miette::Result<()>
where
    F: FnOnce(Context) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<()>> + Send + 'static,
{
    debug!("running '{}' asynchronously", command_name);
    let res = embedded_node(opts, |ctx| {
        async move { f(ctx).await }.with_context(OpenTelemetryContext::current_context())
    });
    local_cmd(res)
}

pub fn embedded_node<F, Fut, T, E>(opts: CommandGlobalOpts, f: F) -> core::result::Result<T, E>
where
    F: FnOnce(Context) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = core::result::Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + Sync + From<CliStateError> + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new()
        .no_logging()
        .with_runtime(opts.rt.clone())
        .build();
    let res = executor.execute(
        async move {
            let child_ctx = ctx
                .new_detached(
                    Address::random_tagged("Detached.embedded_node"),
                    DenyAll,
                    DenyAll,
                )
                .await
                .expect("Embedded node child ctx can't be created");
            let r = f(child_ctx).await;
            let _ = ctx.stop().await;
            r
        }
        .with_context(OpenTelemetryContext::current_context()),
    );
    opts.force_flush();
    match res {
        Ok(Err(e)) => Err(e),
        Ok(Ok(t)) => Ok(t),
        Err(e) => Err(CliStateError::Ockam(e).into()),
    }
}

pub fn embedded_node_that_is_not_stopped<F, Fut, T>(rt: Arc<Runtime>, f: F) -> miette::Result<T>
where
    F: FnOnce(Context) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().with_runtime(rt).build();
    let res = executor.execute(async move {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Detached.embedded_node.not_stopped"),
                DenyAll,
                DenyAll,
            )
            .await
            .expect("Embedded node child ctx can't be created");
        let result = f(child_ctx).await;
        let _ = ctx.stop().await;
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
                    .ok_or(miette!("No transport API has been set on the node"))?;
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

pub fn port_is_free_guard(address: &SocketAddr) -> Result<()> {
    let port = address.port();
    let ip = address.ip();
    if TcpListener::bind((ip, port)).is_err() {
        Err(miette!(
            "Another process is already listening on port {port}!"
        ))?;
    }
    Ok(())
}

pub fn print_deprecated_warning(opts: &CommandGlobalOpts, old: &str, new: &str) -> Result<()> {
    opts.terminal.write_line(fmt_warn!(
        "{} is deprecated. Please use {} instead",
        color_primary(old),
        color_primary(new)
    ))?;
    Ok(())
}

pub fn print_deprecated_flag_warning(opts: &CommandGlobalOpts, deprecated: &str) -> Result<()> {
    opts.terminal.write_line(fmt_warn!(
        "{} is deprecated. This flag has no effect",
        color_primary(deprecated),
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[ockam_macros::test(crate = "ockam")]
    async fn test_process_multi_addr(_ctx: &mut Context) -> ockam::Result<()> {
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
        Ok(())
    }

    #[test]
    fn test_execute_error() {
        let result = embedded_node_that_is_not_stopped(
            Arc::new(Runtime::new().unwrap()),
            |ctx| async move { function_returning_an_error(ctx, 1).await },
        );
        assert!(result.is_err());

        async fn function_returning_an_error(_ctx: Context, _parameter: u8) -> miette::Result<()> {
            Err(miette!("boom"))
        }
    }

    #[test]
    fn test_execute_error_() {
        let result = embedded_node_that_is_not_stopped(
            Arc::new(Runtime::new().unwrap()),
            |ctx| async move { function_returning_an_error_and_stopping_the_context(ctx, 1).await },
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
