use colorful::Colorful;
use miette::miette;

use ockam_api::cloud::operation::Operations;
use ockam_api::cloud::project::{Project, ProjectsOrchestratorApi};
use ockam_api::colors::OckamColor;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_para};
use ockam_node::Context;

use crate::CommandGlobalOpts;

pub async fn check_for_project_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    project: Project,
) -> miette::Result<Project> {
    let pb = opts.terminal.spinner();
    if let Some(spinner) = pb.as_ref() {
        let message = format!(
            "Configuring project...\n{}\n{}",
            fmt_log!("This usually takes 15 seconds, but may sometimes take up to 4 minutes."),
            fmt_log!(
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

    if let Some(spinner) = pb.as_ref() {
        spinner.finish_and_clear();
    }

    Ok(project)
}

#[allow(unused)]
pub async fn check_for_operation_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    operation_id: &str,
    operation_name: &str,
) -> miette::Result<()> {
    let pb = opts.terminal.spinner();
    if let Some(spinner) = pb.as_ref() {
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

    if let Some(spinner) = pb.as_ref() {
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
