use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{debug, info};

use ockam::Context;
use ockam_api::nodes::BackgroundNode;

use crate::node::show::is_node_up;
use crate::node::util::spawn_node;
use crate::node::{guard_node_is_not_already_running, CreateCommand};
use crate::terminal::OckamColor;
use crate::CommandGlobalOpts;
use crate::{color, fmt_log, fmt_ok};

// Create a new node running in the background (i.e. another, new OS process)
pub(crate) async fn background_mode(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    guard_node_is_not_already_running(&opts, &cmd).await?;

    let node_name = cmd.node_name.clone();
    debug!("create node in background mode");

    opts.terminal.write_line(&fmt_log!(
        "Creating Node {}...\n",
        color!(&node_name, OckamColor::PrimaryResource)
    ))?;

    if cmd.child_process {
        return Err(miette!(
            "Cannot create a background node from another background node"
        ));
    }

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        spawn_background_node(&opts, cmd.clone()).await?;
        let mut node = BackgroundNode::create_to_node(&ctx, &opts.state, &node_name).await?;
        let is_node_up = is_node_up(&ctx, &mut node, true).await?;
        *is_finished.lock().await = true;
        Ok(is_node_up)
    };

    let output_messages = vec![
        format!("Creating node..."),
        format!("Starting services..."),
        format!("Loading any pre-trusted identities..."),
    ];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (_response, _) = try_join!(send_req, progress_output)?;

    opts.clone()
        .terminal
        .stdout()
        .plain(
            fmt_ok!(
                "Node {} created successfully\n\n",
                node_name.color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!("To see more details on this node, run:\n")
                + &fmt_log!(
                    "{}",
                    "ockam node show".color(OckamColor::PrimaryResource.color())
                ),
        )
        .write_line()?;

    Ok(())
}

pub(crate) async fn spawn_background_node(
    opts: &CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let trust_context = match cmd.trust_context_opts.trust_context.clone() {
        Some(tc) => {
            let trust_context = opts.state.get_trust_context(&tc).await?;
            Some(trust_context)
        }
        None => None,
    };

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    info!("spawning a new node {}", &cmd.node_name);
    spawn_node(
        opts,
        &cmd.node_name,
        &cmd.identity,
        &cmd.tcp_listener_address,
        cmd.trusted_identities.as_ref(),
        cmd.trusted_identities_file.as_ref(),
        cmd.reload_from_trusted_identities_file.as_ref(),
        cmd.launch_config
            .as_ref()
            .map(|config| serde_json::to_string(config).unwrap()),
        cmd.credential.as_ref(),
        trust_context.as_ref(),
        cmd.trust_context_opts.project_name.clone(),
        cmd.logging_to_file(),
    )
    .await?;

    Ok(())
}
