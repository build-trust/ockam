use crate::local_service::PersistentLocalService;
use crate::state::AppState;
use crate::Error;
use miette::{IntoDiagnostic, WrapErr};
use ockam_api::address::extract_address_value;
use ockam_core::Address;
use ockam_transport_tcp::resolve_peer;
use tracing::{debug, info};

/// The default host to use when creating a TCP outlet if the user doesn't specify one.
const DEFAULT_HOST: &str = "localhost";

impl AppState {
    /// Create a Service and relative TCP outlet within the default node.
    pub async fn create_local_service(
        &self,
        service_name: String,
        scheme: Option<String>,
        address: String,
    ) -> crate::Result<()> {
        debug!(%service_name, %address, "Creating a local service");
        let addr = if let Some((host, port)) = address.split_once(':') {
            format!("{host}:{port}")
        } else {
            format!("{DEFAULT_HOST}:{address}")
        };
        let socket_addr = resolve_peer(addr).into_diagnostic().wrap_err(
            "Invalid address. The expected formats are 'host:port', 'ip:port' or 'port'",
        )?;
        let alias = extract_address_value(&service_name).wrap_err("Invalid service address")?;
        let worker_addr: Address = alias.clone().into();
        let node_manager = self.node_manager().await;
        match node_manager
            .create_outlet(
                &self.context(),
                socket_addr,
                worker_addr.clone(),
                Some(alias.clone()),
                true,
                Some(
                    self.create_invitations_access_control(worker_addr.address().to_string())
                        .await?,
                ),
            )
            .await
        {
            Ok(_status) => {
                info!(socket_addr = socket_addr.to_string(), "Outlet created");
                self.model_mut(|m| {
                    m.add_local_service(PersistentLocalService {
                        socket_addr,
                        worker_addr,
                        alias,
                        scheme,
                    })
                })
                .await?;
                self.publish_state().await;
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to create outlet".to_string())),
        }?;

        Ok(())
    }
}
