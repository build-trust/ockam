use miette::{miette, IntoDiagnostic};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam::AsyncTryClone;
use ockam_api::cloud::operation::Operation;
use ockam_api::cloud::ORCHESTRATOR_AWAIT_TIMEOUT_MS;

use crate::util::{api, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;

pub async fn check_for_completion<'a>(
    opts: &CommandGlobalOpts,
    rpc: &Rpc,
    operation_id: &str,
) -> miette::Result<()> {
    let retry_strategy =
        FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);

    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Configuring project...");
    }
    let operation = Retry::spawn(retry_strategy.clone(), || async {
        let mut rpc_clone = rpc.async_try_clone().await.into_diagnostic()?;
        // Handle the operation show request result
        // so we can provide better errors in the case orchestrator does not respond timely
        let result: Result<Operation> = rpc_clone.ask(api::operation::show(operation_id)).await;
        result.and_then(|o| {
            if o.is_completed() {
                Ok(o)
            } else {
                Err(miette!("Operation timed out. Please try again.").into())
            }
        })
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
