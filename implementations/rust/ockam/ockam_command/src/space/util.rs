use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::cloud::space::Spaces;
use ockam_api::cloud::Controller;

use crate::CommandGlobalOpts;

#[allow(dead_code)]
async fn refresh_spaces(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    controller: &Controller,
) -> miette::Result<()> {
    let spaces = controller
        .list_spaces(ctx)
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    for space in spaces {
        opts.state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
    }
    Ok(())
}
