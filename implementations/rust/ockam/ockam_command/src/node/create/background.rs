use colorful::Colorful;
use miette::miette;
use tracing::{debug, instrument};

use ockam::Context;
use ockam_api::cli_state::journeys::{JourneyEvent, NODE_NAME};
use ockam_api::colors::color_primary;
use ockam_api::logs::CurrentSpan;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_warn};
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
        let node_name = self.name.clone();
        debug!(%node_name, "creating node in background mode");
        CurrentSpan::set_attribute(NODE_NAME, node_name.as_str());

        // Early checks
        if self.child_process {
            return Err(miette!(
                "Cannot create a background node from another background node"
            ));
        }
        if !self.skip_is_running_check {
            self.guard_node_is_not_already_running(&opts).await?;
        }
        if let Some(identity_name) = &self.identity {
            opts.state.get_named_identity(identity_name).await?;
        }

        // Create node and wait for it to be up
        let cmd_with_trace_context = CreateCommand {
            opentelemetry_context: self
                .opentelemetry_context
                .clone()
                .or(Some(OpenTelemetryContext::current())),
            ..self.clone()
        };
        cmd_with_trace_context.spawn_background_node(&opts).await?;
        let mut node = BackgroundNodeClient::create_to_node(ctx, &opts.state, &node_name).await?;
        let is_up = is_node_up(ctx, &mut node, true).await?;
        opts.state
            .add_journey_event(
                JourneyEvent::NodeCreated,
                [(NODE_NAME, node_name.clone())].into(),
            )
            .await?;

        // Output
        if !is_up {
            opts.terminal
                .clone()
                .stdout()
                .plain(fmt_warn!(
                    "Node was {} created but is not reachable",
                    color_primary(&node_name)
                ))
                .write_line()?;
        }
        opts.terminal
            .write_line("")?
            .write_line(fmt_log!("To see more details on this Node, run:"))?
            .write_line(fmt_log!(
                "{}",
                color_primary(format!("ockam node show {}", node_name))
            ))?;
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
        spawn_node(opts, self).await?;

        Ok(())
    }
}
