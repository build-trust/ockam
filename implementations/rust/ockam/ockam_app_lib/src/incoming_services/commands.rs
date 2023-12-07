use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use miette::IntoDiagnostic;
use tracing::{debug, info, warn};

use ockam_api::address::get_free_address;
use ockam_api::nodes::service::portals::Inlets;
use ockam_api::ConnectionStatus;
use ockam_core::api::Reply;
use ockam_multiaddr::MultiAddr;

use crate::background_node::BackgroundNodeClient;
use crate::incoming_services::state::{IncomingService, Port};
use crate::state::AppState;

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
                            if let Some(port) = port {
                                service.set_port(port);
                            }
                            service.set_connected(true);
                        }
                    }
                    Err(err) => {
                        warn!(%err, "Failed to refresh TCP inlet for accepted invitation");
                        if let Some(service) = guard.find_mut_by_id(service.id()) {
                            service.set_connected(false);
                        }
                    }
                }
            }

            // the service resources are already cleaned up at this stage, since when it's
            // removed is always also disabled.
            if service.removed() {
                let mut guard = services_arc.write().await;
                guard.remove_by_id(service.id());
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
    ) -> crate::Result<Option<Port>> {
        let inlet_node_name = &service.local_node_name();
        debug!(node = %inlet_node_name, "Checking node status");
        if !service.enabled() {
            debug!(node = %inlet_node_name, "TCP inlet is disabled by the user, deleting the node");
            let _ = self.delete_background_node(inlet_node_name).await;
            return Ok(None);
        }

        if self.is_connected(service, inlet_node_name).await {
            Ok(None)
        } else {
            self.create_inlet(background_node_client, service)
                .await
                .map(Some)
        }
    }

    /// Returns true if the inlet is already connected to the destination node
    /// if any error occurs, it returns false
    async fn is_connected(&self, service: &IncomingService, inlet_node_name: &str) -> bool {
        if self.state().await.get_node(inlet_node_name).await.is_ok() {
            if let Ok(mut inlet_node) = self.background_node(inlet_node_name).await {
                inlet_node.set_timeout(Duration::from_secs(5));
                if let Ok(Reply::Successful(inlet)) = inlet_node
                    .show_inlet(&self.context(), service.inlet_name())
                    .await
                {
                    if inlet.status == ConnectionStatus::Up {
                        debug!(node = %inlet_node_name, alias = %inlet.alias, "TCP inlet is already up");
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Create the tcp-inlet for the accepted invitation
    /// Returns the inlet [`Port`] if successful
    async fn create_inlet(
        &self,
        background_node_client: Arc<dyn BackgroundNodeClient>,
        service: &IncomingService,
    ) -> crate::Result<Port> {
        debug!(
            service_name = service.name(),
            "Creating TCP inlet for accepted invitation"
        );

        let local_node_name = service.local_node_name();

        // skip enrollment if the ticket is referring to one of our own projects
        // this is useful for testing and in the case the user is connecting
        // multiple devices on the same account
        let (trust_context_name, project_name) = match self.match_owned_projects(service).await? {
            Some((trust_context_name, project_name)) => (trust_context_name, Some(project_name)),
            None => {
                background_node_client
                    .projects()
                    .enroll(
                        &local_node_name,
                        &service.enrollment_ticket().hex_encoded()?,
                    )
                    .await?;
                // the node name is the trust context for enrolled nodes
                (local_node_name.clone(), None)
            }
        };

        // Recreate the node using the trust context
        debug!(node = %local_node_name, "Creating node to host TCP inlet");
        let _ = self.delete_background_node(&local_node_name).await;
        background_node_client
            .nodes()
            .create(&local_node_name, &trust_context_name)
            .await?;
        tokio::time::sleep(Duration::from_millis(250)).await;

        let mut inlet_node = self.background_node(&local_node_name).await?;
        inlet_node.set_timeout(Duration::from_secs(5));

        let bind_address = match service.address() {
            Some(address) => address,
            None => get_free_address()?,
        };

        inlet_node
            .create_inlet(
                &self.context(),
                &bind_address.to_string(),
                &MultiAddr::from_str(&service.service_route(project_name.as_deref()))
                    .into_diagnostic()?,
                &Some(service.inlet_name().to_string()),
                &None,
                Duration::from_secs(5),
            )
            .await
            .map_err(|err| {
                warn!(
                    "Failed to create TCP inlet for accepted invitation: {}",
                    err
                );
                err
            })?;
        Ok(bind_address.port())
    }

    /// Returns the trust context name and project name if one of user own projects matches with
    /// the project in the enrollment ticket
    async fn match_owned_projects(
        &self,
        service: &IncomingService,
    ) -> crate::Result<Option<(String, String)>> {
        let ticket_project = service
            .enrollment_ticket()
            .project
            .as_ref()
            .ok_or_else(|| {
                format!(
                    "The enrollment ticket for the accepted invitation {} should have a project",
                    service.name()
                )
            })?;

        let state = self.state().await;

        let user_email = state.get_default_user().await?.email;
        if let Some(my_project) = state
            .get_projects()
            .await?
            .iter()
            .filter(|p|
                // filter out projects that are not owned by the user
                p.has_admin_with_email(&user_email))
            .find(|p| p.id == ticket_project.id)
        {
            debug!(
                "Skipping enrollment, the project {} is owned by the user",
                my_project.name
            );
            // the project name is also the trust context name
            Ok(Some((my_project.name.clone(), my_project.name.clone())))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn disable_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        // first change the in-memory state
        let service = {
            // we want to reduce the scope of the guard as much as possible
            let incoming_services_arc = self.incoming_services();
            let mut writer = incoming_services_arc.write().await;
            let mut service = writer.find_mut_by_id(invitation_id);
            if let Some(service) = service.as_mut() {
                if !service.enabled() {
                    debug!(node = %service.local_node_name(), alias = %service.name(), "TCP inlet was already disconnected");
                    return Ok(());
                }
                service.disable();
                service.clone()
            } else {
                return Ok(());
            }
        };

        // this is an async operation, let's give feedback to the user as soon as possible
        self.publish_state().await;
        // change the persistent state
        self.model_mut(|model| {
            let service = model.upsert_incoming_service(invitation_id);
            service.enabled = false;
        })
        .await?;
        self.background_node(&service.local_node_name())
            .await?
            .delete_inlet(&self.context(), service.inlet_name())
            .await?;

        Ok(())
    }

    pub(crate) async fn enable_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        // first change the in-memory state
        let changed = {
            // we want to reduce the scope of the guard as much as possible
            let incoming_services_arc = self.incoming_services();
            let mut writer = incoming_services_arc.write().await;
            let mut service = writer.find_mut_by_id(invitation_id);
            if let Some(service) = service.as_mut() {
                if service.enabled() {
                    debug!(node = %service.local_node_name(), alias = %service.name(), "TCP inlet was already enabled");
                    return Ok(());
                }
                service.enable();
                info!(node = %service.local_node_name(), alias = %service.name(), "Enabled TCP inlet");
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
