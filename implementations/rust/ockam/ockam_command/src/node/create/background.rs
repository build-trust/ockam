use miette::miette;
use tracing::{debug, instrument};

use ockam_api::cli_state::journeys::{JourneyEvent, NODE_NAME};
use ockam_api::logs::CurrentSpan;
use ockam_core::OpenTelemetryContext;

use crate::node::node_callback::NodeCallback;
use crate::node::util::spawn_node;
use crate::node::CreateCommand;
use crate::CommandGlobalOpts;

impl CreateCommand {
    // Create a new node running in the background (i.e. another, new OS process)
    #[instrument(skip_all)]
    pub(crate) async fn background_mode(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node_name = self.name.clone();
        debug!(%node_name, "creating node in background mode");
        CurrentSpan::set_attribute(NODE_NAME, node_name.as_str());

        if self.foreground_args.child_process {
            return Err(miette!(
                "Cannot create a background node from another background node"
            ));
        }

        let node_callback = NodeCallback::create().await?;

        // Create node and wait for it to be up and configured
        let cmd = CreateCommand {
            opentelemetry_context: self
                .opentelemetry_context
                .clone()
                .or(Some(OpenTelemetryContext::current())),
            tcp_callback_port: Some(node_callback.callback_port()),
            ..self.clone()
        };

        // Run foreground node in a separate process
        // Output is handled in the foreground execution
        let handle = spawn_node(&opts, cmd)?;

        tokio::select! {
            _ = handle.wait_with_output() => { std::process::exit(1) }
            _ = node_callback.wait_for_signal() => {}
        }

        opts.state
            .add_journey_event(JourneyEvent::NodeCreated, [(NODE_NAME, node_name)].into())
            .await?;

        Ok(())
    }
}
