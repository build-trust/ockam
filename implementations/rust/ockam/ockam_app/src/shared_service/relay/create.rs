use crate::app::AppState;
use crate::Result;

pub async fn create_relay(_app_state: &AppState) -> Result<()> {
    // TODO: the relay creation fails because the NodeManagerWorker is initialized before the
    //       enrollment, hence, it's not initialized with the project/trust-context info.
    //       We need to figure out how to update the NodeManagerWorker after the enrollment is done.
    // let project_route = app_state
    //     .state()
    //     .projects
    //     .default()?
    //     .config()
    //     .access_route
    //     .clone();
    // let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
    // let req = CreateForwarder::at_project(project_address.clone(), None);
    // app_state
    //     .node_manager
    //     .create_forwarder(&app_state.context(), req)
    //     .await
    //     .into_diagnostic()?;
    Ok(())
}
