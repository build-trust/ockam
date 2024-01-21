use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decoder, Encode};
use tokio::time::{sleep, Duration};
use tracing::debug;

use ockam::{Address, AsyncTryClone, TcpListenerOptions};
use ockam::{Context, TcpTransport};
use ockam_api::nodes::service::NodeManagerTrustOptions;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{
    bootstrapped_identities_store::PreTrustedIdentities,
    nodes::{
        service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
        NodeManagerWorker, NODEMANAGER_ADDR,
    },
};
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::{route, LOCAL};

use crate::fmt_ok;
use crate::node::{guard_node_is_not_already_running, CreateCommand};
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::util::api;
use crate::{shutdown, CommandGlobalOpts, Result};

pub(super) async fn foreground_mode(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    guard_node_is_not_already_running(&opts, &cmd).await?;

    let node_name = cmd.node_name.clone();
    debug!("create node {node_name} in foreground mode");

    if opts
        .state
        .get_node(&node_name)
        .await
        .ok()
        .map(|n| n.is_running())
        .unwrap_or(false)
    {
        return Err(miette!("Node {} is already running", &node_name));
    };

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&cmd.tcp_listener_address, options)
        .await
        .into_diagnostic()?;

    debug!(
        "set the node {node_name} listener address to {:?}",
        listener.socket_address()
    );

    // Set node_name so that node can isolate its data in the storage from other nodes
    let mut state = opts.state.clone();
    state.set_node_name(node_name.clone());

    let node_info = state
        .start_node_with_optional_values(
            &node_name,
            &cmd.identity,
            &cmd.trust_context_opts.project_name,
            Some(&listener),
        )
        .await?;
    debug!("created node {node_info:?}");

    let named_trust_context = state
        .retrieve_trust_context(
            &cmd.trust_context_opts.trust_context,
            &cmd.trust_context_opts.project_name,
            &cmd.authority_identity().await?,
            &cmd.credential,
        )
        .await?;

    let pre_trusted_identities = load_pre_trusted_identities(&cmd)?;

    let node_man = InMemoryNode::new(
        &ctx,
        NodeManagerGeneralOptions::new(
            state,
            node_name.clone(),
            pre_trusted_identities,
            cmd.launch_config.is_none(),
            true,
        ),
        NodeManagerTransportOptions::new(
            listener.flow_control_id().clone(),
            tcp.async_try_clone().await.into_diagnostic()?,
        ),
        NodeManagerTrustOptions::new(named_trust_context),
    )
    .await
    .into_diagnostic()?;
    let node_manager_worker = NodeManagerWorker::new(Arc::new(node_man));

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await
        .into_diagnostic()?;

    if let Some(config) = &cmd.launch_config {
        if start_services(&ctx, config).await.is_err() {
            //TODO: Process should terminate on any error during its setup phase,
            //      not just during the start_services.
            //TODO: This sleep here is a workaround on some orchestrated environment,
            //      the lmdb db, that is used for policy storage, fails to be re-opened
            //      if it's still opened from another docker container, where they share
            //      the same pid. By sleeping for a while we let this container be promoted
            //      and the other being terminated, so when restarted it works.  This is
            //      FAR from ideal.
            sleep(Duration::from_secs(10)).await;
            ctx.stop().await.into_diagnostic()?;
            return Err(miette!("Failed to start services"));
        }
    }

    // Create a channel for communicating back to the main thread
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    shutdown::wait(
        opts.terminal.clone(),
        cmd.exit_on_eof,
        opts.global_args.quiet,
        tx,
        &mut rx,
    )
    .await?;

    // Try to stop node; it might have already been stopped or deleted (e.g. when running `node delete --all`)
    opts.state.stop_node(&node_name, true).await?;
    ctx.stop().await.into_diagnostic()?;
    opts.terminal
        .write_line(fmt_ok!("Node stopped successfully"))
        .unwrap();

    Ok(())
}

pub fn load_pre_trusted_identities(cmd: &CreateCommand) -> Result<Option<PreTrustedIdentities>> {
    let command = cmd.clone();
    let pre_trusted_identities = match (
        command.trusted_identities,
        command.trusted_identities_file,
        command.reload_from_trusted_identities_file,
    ) {
        (Some(val), _, _) => Some(PreTrustedIdentities::new_from_string(&val)?),
        (_, Some(val), _) => Some(PreTrustedIdentities::new_from_disk(val, false)?),
        (_, _, Some(val)) => Some(PreTrustedIdentities::new_from_disk(val, true)?),
        _ => None,
    };
    Ok(pre_trusted_identities)
}

async fn start_services(ctx: &Context, cfg: &Config) -> miette::Result<()> {
    let config = {
        if let Some(sc) = &cfg.startup_services {
            sc.clone()
        } else {
            return Ok(());
        }
    };

    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let identity = cfg.identity;
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, identity, route![]).await?;
        }
    }
    if let Some(cfg) = config.authenticator {
        if !cfg.disabled {
            println!("starting authenticator service ...");
            let req = api::start_authenticator_service(&cfg.address, &cfg.project);
            send_req_to_node_manager(ctx, req).await?;
        }
    }
    if let Some(cfg) = config.okta_identity_provider {
        if !cfg.disabled {
            println!("starting okta identity provider service ...");
            let req = api::start_okta_service(&cfg);
            send_req_to_node_manager(ctx, req).await?;
        }
    }

    Ok(())
}

async fn send_req_to_node_manager<T>(ctx: &Context, req: Request<T>) -> Result<()>
where
    T: Encode<()>,
{
    let buf: Vec<u8> = ctx
        .send_and_receive(NODEMANAGER_ADDR, req.to_vec()?)
        .await?;
    let mut dec = Decoder::new(&buf);
    let hdr = dec.decode::<ResponseHeader>()?;
    if hdr.status() != Some(Status::Ok) {
        return Err(miette!("Request failed with status: {:?}", hdr.status()))?;
    }
    Ok(())
}
