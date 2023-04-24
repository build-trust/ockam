use anyhow::anyhow;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam_api::cloud::operation::Operation;

use crate::util::api::CloudOpts;
use crate::util::{api, RpcBuilder};
use crate::{CommandGlobalOpts, Result};

pub async fn check_for_completion<'a>(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    api_node: &str,
    operation_id: &str,
) -> Result<()> {
    let total_sleep_time_ms = 10 * 60 * 1000;
    let retry_strategy = FixedInterval::from_millis(5000).take(total_sleep_time_ms / 5000);

    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Configuring project (this can take a few minutes) ...");
    }

    let operation = Retry::spawn(retry_strategy.clone(), || async {
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();

        // Handle the operation show request result
        // so we can provide better errors in the case orchestrator does not respond timely
        if rpc
            .request(api::operation::show(operation_id, &cloud_opts.route()))
            .await
            .is_ok()
        {
            let operation = rpc.parse_response::<Operation>()?;
            if operation.is_completed() {
                return Ok(operation.to_owned());
            }
        }
        Err(anyhow!("Operation timed out. Please try again."))
    })
    .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.finish_and_clear();
    }

    if operation.is_successful() {
        Ok(())
    } else {
        Err(anyhow!("Operation failed. Please try again.").into())
    }
}
