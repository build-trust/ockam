use std::collections::HashMap;

use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{debug, info, instrument};

use ockam::Context;
use ockam_api::cli_state::journeys::{JourneyEvent, NODE_NAME};
use ockam_api::colors::OckamColor;
use ockam_api::logs::CurrentSpan;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{color, fmt_log, fmt_ok};
use ockam_core::OpenTelemetryContext;

use crate::node::show::is_node_up;
use crate::node::util::spawn_node;
use crate::node::CreateCommand;
use crate::CommandGlobalOpts;

impl CreateCommand {
    // Create a new node running in the background (i.e. another, new OS process)
    #[instrument(skip_all)]
    pub(crate) async fn background_mode(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.skip_is_running_check {
            self.guard_node_is_not_already_running(&opts).await?;
        }

        if let Some(identity_name) = &self.identity {
            opts.state.get_named_identity(identity_name).await?;
        }

        let node_name = self.name.clone();
        CurrentSpan::set_attribute(NODE_NAME, node_name.as_str());
        debug!("create node in background mode");

        opts.terminal.write_line(&fmt_log!(
            "Creating Node {}...\n",
            color!(&node_name, OckamColor::PrimaryResource)
        ))?;

        if self.child_process {
            return Err(miette!(
                "Cannot create a background node from another background node"
            ));
        }

        let is_finished: Mutex<bool> = Mutex::new(false);

        let opentelemetry_context = OpenTelemetryContext::current();
        let cmd_with_trace_context = CreateCommand {
            opentelemetry_context: self
                .opentelemetry_context
                .clone()
                .or(Some(opentelemetry_context)),
            ..self.clone()
        };

        let send_req = async {
            cmd_with_trace_context.spawn_background_node(&opts).await?;
            let mut node =
                BackgroundNodeClient::create_to_node(ctx, &opts.state, &node_name).await?;
            let is_node_up = is_node_up(ctx, &mut node, true).await?;
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

        let mut attributes = HashMap::new();
        attributes.insert(NODE_NAME, node_name.clone());
        opts.state
            .add_journey_event(JourneyEvent::NodeCreated, attributes)
            .await?;

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
        self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.skip_is_running_check {
            self.guard_node_is_not_already_running(opts).await?;
        }

        // Construct the argument list and re-execute the ockam
        // CLI in foreground mode to start the newly created node
        info!("spawning a new node {}", &self.name);
        spawn_node(opts, self).await?;

        Ok(())
    }
}
