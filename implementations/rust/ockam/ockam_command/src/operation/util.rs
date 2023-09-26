use miette::miette;
use std::sync::Arc;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam_api::cloud::operation::Operations;
use ockam_api::cloud::{Controller, ORCHESTRATOR_AWAIT_TIMEOUT_MS};
use ockam_node::Context;

use crate::CommandGlobalOpts;

pub async fn check_for_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    controller: Arc<Controller>,
    operation_id: &str,
) -> miette::Result<()> {
    let retry_strategy =
        FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);

    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Configuring project...");
    }
    let operation = Retry::spawn(retry_strategy.clone(), || async {
        // Handle the operation show request result
        // so we can provide better errors in the case orchestrator does not respond timely
        let result = controller.get_operation(ctx, operation_id).await?;

        match result {
            Some(o) if o.is_completed() => Ok(o),
            _ => Err(miette!("Operation timed out. Please try again.")),
        }
    })
    .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.finish_and_clear();
    }

    if operation.is_successful() {
        Ok(())
    } else {
        Err(miette!("Operation failed. Please try again."))
    }
}
