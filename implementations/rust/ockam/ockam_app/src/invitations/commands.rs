use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, info};

use ockam_api::cloud::share::{AcceptInvitation, InvitationListKind, ListInvitations};
use ockam_command::util::api::CloudOpts;

use crate::app::AppState;

use super::{
    events::REFRESHED_INVITATIONS,
    state::{InvitationState, SyncState},
};

// At time of writing, tauri::command requires pub not pub(crate)

#[tauri::command]
pub async fn accept_invitation<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), String> {
    info!(?id, "accepting invitation");
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let res = node_manager_worker
        .accept_invitation(
            &state.context(),
            AcceptInvitation { id },
            &CloudOpts::route(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    debug!(?res);
    app.trigger_global(super::events::REFRESH_INVITATIONS, None);
    Ok(())
}

#[tauri::command]
pub async fn list_invitations<R: Runtime>(app: AppHandle<R>) -> tauri::Result<InvitationState> {
    let state: State<'_, SyncState> = app.state();
    let reader = state.read().await;
    Ok((*reader).clone())
}

#[tauri::command]
pub async fn refresh_invitations<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("refreshing invitations");
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let invitations = node_manager_worker
        .list_shares(
            &state.context(),
            ListInvitations {
                kind: InvitationListKind::All,
            },
            &CloudOpts::route(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    debug!(?invitations);
    {
        let invitation_state: State<'_, SyncState> = app.state();
        let mut writer = invitation_state.write().await;
        *writer = invitations.into();
    }
    app.trigger_global(REFRESHED_INVITATIONS, None);
    Ok(())
}
