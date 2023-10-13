use crate::state::AppState;
use crate::Error;
use miette::{IntoDiagnostic, WrapErr};
use ockam_api::address::extract_address_value;
use ockam_transport_tcp::resolve_peer;
use tracing::{debug, info};

/// The default host to use when creating a TCP outlet if the user doesn't specify one.
const DEFAULT_HOST: &str = "localhost";

impl AppState {
    /// Create a TCP outlet within the default node.
    pub async fn tcp_outlet_create(
        &self,
        service: String,
        address: String,
        emails: Vec<String>,
    ) -> crate::Result<()> {
        debug!(%service, %address, "Creating an outlet");
        let addr = if let Some((host, port)) = address.split_once(':') {
            format!("{host}:{port}")
        } else {
            format!("{DEFAULT_HOST}:{address}")
        };
        let socket_addr = resolve_peer(addr).into_diagnostic().wrap_err(
            "Invalid address. The expected formats are 'host:port', 'ip:port' or 'port'",
        )?;
        let worker_addr = extract_address_value(&service).wrap_err("Invalid service address")?;
        let node_manager = self.node_manager().await;
        match node_manager
            .create_outlet(
                &self.context(),
                socket_addr,
                worker_addr.clone().into(),
                Some(worker_addr),
                true,
            )
            .await
        {
            Ok(status) => {
                info!(socket_addr = socket_addr.to_string(), "Outlet created");
                self.model_mut(|m| m.add_tcp_outlet(status)).await?;
                self.publish_state().await;
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to create outlet".to_string())),
        }?;

        for email in emails {
            self.create_service_invitation_by_socket_addr(email, socket_addr.to_string())
                .await?;
        }
        Ok(())
    }
}
