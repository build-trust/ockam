use miette::{IntoDiagnostic, WrapErr};
use ockam_api::address::extract_address_value;
use ockam_transport_tcp::resolve_peer;
use tauri::{AppHandle, Manager, Runtime};
use tracing::{debug, error, info};

use crate::app::events::system_tray_on_update;
use crate::app::AppState;
use crate::invitations::commands::create_service_invitation;
use crate::Error;

/// The default host to use when creating a TCP outlet if the user doesn't specify one.
const DEFAULT_HOST: &str = "localhost";

/// Create a TCP outlet within the default node.
#[tauri::command]
pub async fn tcp_outlet_create<R: Runtime>(
    app: AppHandle<R>,
    service: String,
    address: String,
    email: String,
) -> Result<(), String> {
    let email = if email.is_empty() { None } else { Some(email) };
    tcp_outlet_create_impl(app, service, address, email)
        .await
        .map_err(|e| {
            error!("{:?}", e);
            e.to_string()
        })?;
    Ok(())
}

async fn tcp_outlet_create_impl<R: Runtime>(
    app: AppHandle<R>,
    service: String,
    address: String,
    email: Option<String>,
) -> crate::Result<()> {
    debug!(%service, %address, "Creating an outlet");
    let app_state = app.state::<AppState>();
    let addr = if let Some((host, port)) = address.split_once(':') {
        format!("{host}:{port}")
    } else {
        format!("{DEFAULT_HOST}:{address}")
    };
    let socket_addr = resolve_peer(addr)
        .into_diagnostic()
        .wrap_err("Invalid address. The expected formats are 'host:port', 'ip:port' or 'port'")?;
    let worker_addr = extract_address_value(&service).wrap_err("Invalid service address")?;
    let node_manager_worker = app_state.node_manager_worker().await;
    match node_manager_worker
        .node_manager
        .create_outlet(
            &app_state.context(),
            socket_addr,
            worker_addr.into(),
            None,
            true,
        )
        .await
    {
        Ok(status) => {
            info!(socket_addr = socket_addr.to_string(), "Outlet created");
            app_state.model_mut(|m| m.add_tcp_outlet(status)).await?;
            system_tray_on_update(&app);
            Ok(())
        }
        Err(_) => Err(Error::App("Failed to create outlet".to_string())),
    }?;

    if let Some(email) = email {
        create_service_invitation(email, socket_addr.to_string(), app).await?;
    }
    Ok(())
}
