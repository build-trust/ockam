use colorful::Colorful;
use miette::miette;
use ockam_api::cloud::operation::Operations;
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

use crate::fmt_para;
use crate::terminal::OckamColor;
use crate::CommandGlobalOpts;

pub async fn check_for_project_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    project: Project,
) -> miette::Result<Project> {
    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        let message = format!(
            "Configuring project...\n{}\n{}",
            fmt_para!("This may take 2 to 4 minutes."),
            fmt_para!(
                "{}",
                "Please do not press Ctrl+C or exit the terminal process until this is complete."
                    .to_string()
                    .color(OckamColor::FmtWARNBackground.color())
            ),
        );
        spinner.set_message(message);
    }
    let project = node
        .wait_until_project_creation_operation_is_complete(ctx, project)
        .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.finish_and_clear();
    }
    Ok(project)
}

pub async fn check_for_operation_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    operation_id: &str,
    operation_name: &str,
) -> miette::Result<()> {
    let spinner_option = opts.terminal.progress_spinner();
    if let Some(spinner) = spinner_option.as_ref() {
        let message = format!(
            "Waiting for {operation_name} to finish ...\n{}",
            fmt_para!(
                "{}",
                "Please do not press Ctrl+C or exit the terminal process until this is complete."
                    .to_string()
                    .color(OckamColor::FmtWARNBackground.color())
            ),
        );
        spinner.set_message(message);
    }
    let result = node
        .wait_until_operation_is_complete(ctx, operation_id)
        .await;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.finish_and_clear();
    }

    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(miette!(
            "The operation {} ({}) was not successful: {}. Please try again.",
            operation_name,
            operation_id,
            e
        )),
    }
}
