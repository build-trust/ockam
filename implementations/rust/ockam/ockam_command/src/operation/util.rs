use colorful::Colorful;
use miette::miette;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam_api::cloud::operation::Operations;
use ockam_api::cloud::{Controller, ORCHESTRATOR_AWAIT_TIMEOUT_MS};
use ockam_node::Context;

use crate::fmt_para;
use crate::terminal::OckamColor;
use crate::CommandGlobalOpts;

pub async fn check_for_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    controller: &Controller,
    operation_id: &str,
) -> miette::Result<()> {
    let retry_strategy =
        FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);

    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        let message = format!(
            "Configuring project...\n{}\n{}",
            fmt_para!("This takes about 2 minutes."),
            fmt_para!(
                "{}",
                "Do not press Ctrl+C or exit the terminal process until this is complete."
                    .to_string()
                    .color(OckamColor::FmtWARNBackground.color())
            ),
        );
        spinner.set_message(message);
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
