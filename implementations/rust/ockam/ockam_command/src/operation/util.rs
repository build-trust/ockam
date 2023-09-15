use miette::{miette, IntoDiagnostic};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam_api::cloud::operation::{Operation, Operations};
use ockam_api::cloud::ORCHESTRATOR_AWAIT_TIMEOUT_MS;
use ockam_core::api::Reply;
use ockam_node::Context;

use crate::node::util::LocalNode;
use crate::CommandGlobalOpts;

pub async fn check_for_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &LocalNode,
    operation_id: &str,
) -> miette::Result<()> {
    let retry_strategy =
        FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);

    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Configuring project...");
    }
    let operation: Operation = Retry::spawn(retry_strategy.clone(), || async {
        // Handle the operation show request result
        // so we can provide better errors in the case orchestrator does not respond timely
        let result = node
            .get_operation(ctx, operation_id)
            .await
            .into_diagnostic()?;
        let operation: miette::Result<Operation> = match result {
            Reply::Successful(o) if o.is_completed() => Ok(o),
            _ => Err(miette!("Operation timed out. Please try again.")),
        };
        operation
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
