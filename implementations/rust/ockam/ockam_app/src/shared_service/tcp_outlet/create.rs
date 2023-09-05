use miette::{IntoDiagnostic, WrapErr};
use ockam_api::address::extract_address_value;
use std::net::SocketAddr;
use tauri::{AppHandle, Manager, Wry};
use tracing::{debug, error, info};

use crate::app::events::system_tray_on_update;
use crate::app::AppState;
use crate::error::Error;
use crate::invitations::commands::create_service_invitation;

/// Create a TCP outlet within the default node.
#[tauri::command]
pub async fn tcp_outlet_create(
    app: AppHandle<Wry>,
    service: String,
    port: String,
    email: String,
) -> Result<(), String> {
    let email = if email.is_empty() { None } else { Some(email) };
    tcp_outlet_create_impl(app, service, port, email)
        .await
        .map_err(|e| {
            error!("{:?}", e);
            e.to_string()
        })?;
    Ok(())
}

async fn tcp_outlet_create_impl(
    app: AppHandle<Wry>,
    service: String,
    port: String,
    email: Option<String>,
) -> crate::Result<()> {
    debug!(%service, %port, "Creating an outlet");
    let app_state = app.state::<AppState>();
    let socket_addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .into_diagnostic()
        .wrap_err("Invalid port")?;
    let worker_addr = extract_address_value(&service).wrap_err("Invalid service address")?;
    let node_manager_worker = app_state.node_manager_worker().await;
    let mut node_manager = node_manager_worker.inner().write().await;
    match node_manager
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
        Err(e) => Err(Error::App(format!("Failed to create service: {}", e))),
    }?;

    if let Some(email) = email {
        create_service_invitation(email, socket_addr.to_string(), app).await?;
    }
    Ok(())
}
