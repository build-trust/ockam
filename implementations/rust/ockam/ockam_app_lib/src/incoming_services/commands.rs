use crate::background_node::BackgroundNodeClient;
use crate::incoming_services::state::IncomingService;
use crate::state::AppState;
use miette::IntoDiagnostic;
use ockam_api::address::get_free_address;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::service::portals::Inlets;
use ockam_api::ConnectionStatus;
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

impl AppState {
    pub(crate) async fn refresh_inlets(&self) -> crate::Result<()> {
        info!("Refreshing inlets");

        // for each invitation it checks if the relative node is running
        // if not, it deletes the node and re-create the inlet

        let services_arc = self.incoming_services();
        let services = {
            // reduce locking as much as possible to make UI consistently responsive
            services_arc.read().await.services.clone()
        };
        if services.is_empty() {
            debug!("No incoming services, skipping inlets refresh");
            return Ok(());
        }

        let background_node_client = self.background_node_client().await;
        for service in services {
            let result = self
                .refresh_inlet(background_node_client.clone(), &service)
                .await;
            {
                // we want to reduce the scope of the guard as much as possible
                let mut guard = services_arc.write().await;
                match result {
                    Ok(port) => {
                        if let Some(service) = guard.find_mut_by_id(service.id()) {
                            service.set_port(port)
                        }
                    }
                    Err(err) => {
                        warn!(%err, "Failed to refresh TCP inlet for accepted invitation");
                        if let Some(service) = guard.find_mut_by_id(service.id()) {
                            service.set_port(None)
                        }
                    }
                }
            }
            self.publish_state().await;
        }

        info!("Inlets refreshed");
        Ok(())
    }

    async fn refresh_inlet(
        &self,
        background_node_client: Arc<dyn BackgroundNodeClient>,
        service: &IncomingService,
    ) -> crate::Result<Option<u16>> {
        let inlet_node_name = &service.local_node_name()?;
        debug!(node = %inlet_node_name, "Checking node status");
        if !service.enabled() {
            debug!(node = %inlet_node_name, "TCP inlet is disabled by the user, deleting the node");
            let _ = self.delete_background_node(inlet_node_name).await;
            return Ok(None);
        }

        if self.state().await.nodes.exists(inlet_node_name) {
            let mut inlet_node = self.background_node(inlet_node_name).await?;
            inlet_node.set_timeout(Duration::from_secs(5));

            if let Ok(inlet) = inlet_node
                .show_inlet(&self.context(), service.inlet_name())
                .await?
                .success()
            {
                if inlet.status == ConnectionStatus::Up {
                    debug!(node = %inlet_node_name, alias = %inlet.alias, "TCP inlet is already up");
                    let bind_address: SocketAddr = inlet.bind_addr.parse()?;
                    return Ok(Some(bind_address.port()));
                }
            }
        }

        self.create_inlet(background_node_client, service)
            .await
            .map(Some)
    }

    /// Create the tcp-inlet for the accepted invitation
    /// Returns the inlet SocketAddr
    async fn create_inlet(
        &self,
        background_node_client: Arc<dyn BackgroundNodeClient>,
        service: &IncomingService,
    ) -> crate::Result<u16> {
        debug!(
            service_name = service.name(),
            "Creating TCP inlet for accepted invitation"
        );

        let local_node_name = service.local_node_name()?;

        let bind_address = match service.address() {
            Some(address) => address,
            None => get_free_address()?,
        };

        background_node_client
            .projects()
            .enroll(
                &local_node_name,
                &service.enrollment_ticket()?.hex_encoded()?,
            )
            .await?;

        // Recreate the node using the trust context
        debug!(node = %local_node_name, "Creating node to host TCP inlet");
        let _ = self.delete_background_node(&local_node_name).await;
        background_node_client
            .nodes()
            .create(&local_node_name)
            .await?;
        tokio::time::sleep(Duration::from_millis(250)).await;

        let mut inlet_node = self.background_node(&local_node_name).await?;
        inlet_node.set_timeout(Duration::from_secs(5));
        inlet_node
            .create_inlet(
                &self.context(),
                &bind_address.to_string(),
                &MultiAddr::from_str(&service.service_route()?).into_diagnostic()?,
                &Some(service.inlet_name().to_string()),
                &None,
                Duration::from_secs(5),
            )
            .await?;
        Ok(bind_address.port())
    }

    pub(crate) async fn disable_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        // first change the in-memory state
        let service = {
            let incoming_services_arc = self.incoming_services();
            let mut writer = incoming_services_arc.write().await;
            let mut service = writer.find_mut_by_id(invitation_id);
            if let Some(service) = service.as_mut() {
                if !service.enabled() {
                    debug!(node = %service.local_node_name()?, alias = %service.name(), "TCP inlet was already disconnected");
                    return Ok(());
                }
                service.disable();
            }
            service.cloned()
        };

        if let Some(service) = service {
            // this is an async operation, let's give feedback to the user as soon as possible
            self.publish_state().await;
            // change the persistent state
            self.model_mut(|model| {
                let service = model.upsert_incoming_service(invitation_id);
                service.enabled = false;
            })
            .await?;
            self.background_node(&service.local_node_name()?)
                .await?
                .delete_inlet(&self.context(), service.inlet_name())
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn enable_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        // first change the in-memory state
        let changed = {
            let incoming_services_arc = self.incoming_services();
            let mut writer = incoming_services_arc.write().await;
            let mut service = writer.find_mut_by_id(invitation_id);
            if let Some(service) = service.as_mut() {
                if service.enabled() {
                    debug!(node = %service.local_node_name()?, alias = %service.name(), "TCP inlet was already enabled");
                    return Ok(());
                }
                service.enable();
                info!(node = %service.local_node_name()?, alias = %service.name(), "Enabled TCP inlet");
                true
            } else {
                false
            }
        };

        if changed {
            // this is an async operation, let's give feedback to the user as soon as possible
            self.publish_state().await;
            // change the persistent state
            self.model_mut(|model| {
                let service = model.upsert_incoming_service(invitation_id);
                service.enabled = true;
            })
            .await?;
        }
        Ok(())
    }
}
