use tauri::menu::{IconMenuItemBuilder, MenuBuilder, NativeIcon};
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::debug;

use crate::app::AppState;
use crate::shared_service::relay::get_relay;

pub(crate) async fn build_relay_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    let node_manager = app_state.node_manager().await;
    match get_relay(node_manager).await {
        Some(relay) => {
            debug!(relay = %relay.forwarding_route(), "Relay up and running");
            builder = builder.item(
                &IconMenuItemBuilder::new("Connected to Ockam Orchestrator")
                    .native_icon(NativeIcon::StatusAvailable)
                    .build(app_handle),
            )
        }
        None => {
            debug!("Relay not running");
            if app_state.is_enrolled().await.unwrap_or(false) {
                builder = builder.item(
                    &IconMenuItemBuilder::new("Connecting to Ockam Orchestrator...")
                        .native_icon(NativeIcon::StatusPartiallyAvailable)
                        .enabled(false)
                        .build(app_handle),
                )
            }
        }
    }
    builder
}
