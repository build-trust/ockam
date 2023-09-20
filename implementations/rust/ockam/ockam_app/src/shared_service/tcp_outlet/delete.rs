use tauri::{AppHandle, Manager, Runtime};
use tracing::{debug, error, info};

use crate::app::events::system_tray_on_update;
use crate::app::AppState;
use crate::Error;

/// Delete a TCP outlet from the default node.
#[tauri::command]
pub async fn tcp_outlet_delete<R: Runtime>(app: AppHandle<R>, alias: String) -> Result<(), String> {
    tcp_outlet_delete_impl(app, alias).await.map_err(|e| {
        error!("{:?}", e);
        e.to_string()
    })?;
    Ok(())
}

async fn tcp_outlet_delete_impl<R: Runtime>(app: AppHandle<R>, alias: String) -> crate::Result<()> {
    debug!(%alias, "Deleting a TCP outlet");
    let app_state = app.state::<AppState>();
    let node_manager = app_state.node_manager().await;
    match node_manager.delete_outlet(&alias).await {
        Ok(_) => {
            info!(%alias, "TCP outlet deleted");
            app_state.model_mut(|m| m.delete_tcp_outlet(&alias)).await?;
            system_tray_on_update(&app);
            Ok(())
        }
        Err(_) => Err(Error::App("Failed to delete TCP outlet".to_string())),
    }?;
    Ok(())
}
