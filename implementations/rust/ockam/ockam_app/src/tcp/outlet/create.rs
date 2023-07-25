use ockam_command::util::extract_address_value;
use tauri::{AppHandle, Manager, Wry};
use tracing::{error, info};

use crate::app::AppState;
use crate::tcp::outlet::SERVICE_WINDOW_ID;

/// Create a TCP outlet within the default node.
#[tauri::command]
pub async fn tcp_outlet_create(
    app: AppHandle<Wry>,
    service: String,
    port: String,
) -> tauri::Result<()> {
    info!(%service, %port, "Creating an outlet");
    let state = app.state::<AppState>();
    let tcp_addr = format!("127.0.0.1:{port}")
        .parse()
        .expect("Invalid IP address");
    let worker_addr = extract_address_value(&service).expect("Invalid service address");
    let mut node_manager = state.node_manager.get().write().await;
    let res = node_manager
        .create_outlet(&state.context(), tcp_addr, worker_addr, None, true)
        .await;
    match res {
        Err(e) => error!("{:?}", e),
        Ok(s) => info!("the outlet status is {:?}", s),
    }
    app.get_window(SERVICE_WINDOW_ID).map(|w| w.close());
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}
