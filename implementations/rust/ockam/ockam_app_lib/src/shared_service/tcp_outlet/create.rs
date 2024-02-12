use crate::state::AppState;
use crate::Error;
use miette::{IntoDiagnostic, WrapErr};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::portal::OutletAccessControl;
use ockam_core::Address;
use ockam_transport_tcp::resolve_peer;
use tracing::{debug, info};

/// The default host to use when creating a TCP outlet if the user doesn't specify one.
const DEFAULT_HOST: &str = "localhost";

impl AppState {
    /// Create a TCP outlet within the default node.
    pub async fn tcp_outlet_create(&self, from: String, to: String) -> crate::Result<()> {
        debug!(%from, %to, "Creating an outlet");
        let addr = if let Some((host, port)) = to.split_once(':') {
            format!("{host}:{port}")
        } else {
            format!("{DEFAULT_HOST}:{to}")
        };
        let socket_addr = resolve_peer(addr).into_diagnostic().wrap_err(
            "Invalid address. The expected formats are 'host:port', 'ip:port' or 'port'",
        )?;
        let worker_addr: Address = extract_address_value(&from)
            .wrap_err("Invalid service address")?
            .into();
        let node_manager = self.node_manager().await;
        match node_manager
            .create_outlet(
                &self.context(),
                socket_addr,
                Some(worker_addr.clone()),
                true,
                OutletAccessControl::IncomingAccessControl(
                    self.create_invitations_access_control(worker_addr).await?,
                ),
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

        Ok(())
    }
}
