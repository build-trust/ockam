use miette::{IntoDiagnostic, WrapErr};
use ockam_command::util::extract_address_value;
use std::net::SocketAddr;
use tauri::{AppHandle, Manager, Wry};
use tracing::{debug, error, info};

use crate::app::AppState;
use crate::error::Error;
use crate::invitations::build_args_for_create_service_invitation;
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
    let tcp_addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .into_diagnostic()
        .wrap_err("Invalid port")?;
    let worker_addr = extract_address_value(&service).wrap_err("Invalid service address")?;
    let node_manager_worker = app_state.node_manager_worker().await;
    let mut node_manager = node_manager_worker.inner().write().await;
    match node_manager
        .create_outlet(
            &app_state.context(),
            tcp_addr.to_string(),
            worker_addr,
            None,
            true,
        )
        .await
    {
        Ok(status) => {
            info!(tcp_addr = status.tcp_addr, "Outlet created");
            app_state.model_mut(|m| m.add_tcp_outlet(status)).await?;
            app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
            Ok(())
        }
        Err(_) => Err(Error::Generic("Failed to create outlet".to_string())),
    }?;
    if let Some(email) = email {
        let invite_args =
            build_args_for_create_service_invitation(&app, &tcp_addr.to_string(), &email).await?;
        create_service_invitation(invite_args, app)
            .await
            .map_err(Error::Generic)?;
    }
    Ok(())
}
