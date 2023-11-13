use crate::state::{AppState, ModelState};
use minicbor::{Decode, Encode};
use ockam_api::cloud::share::{InvitationWithAccess, ServiceAccessDetails};
use ockam_api::error::ApiError;
use ockam_api::identity::EnrollmentTicket;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::warn;

#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PersistentIncomingServiceState {
    #[n(1)] pub(crate) invitation_id: String,
    #[n(2)] pub(crate) enabled: bool,
    #[n(3)] pub(crate) name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct IncomingServicesState {
    pub(crate) services: Vec<IncomingService>,
}

impl IncomingServicesState {
    pub(crate) fn find_by_id(&self, id: &str) -> Option<&IncomingService> {
        self.services.iter().find(|s| s.id == id)
    }

    pub(crate) fn find_mut_by_id(&mut self, id: &str) -> Option<&mut IncomingService> {
        self.services.iter_mut().find(|s| s.id == id)
    }
}

impl ModelState {
    pub(crate) fn upsert_incoming_service(
        &mut self,
        id: &str,
    ) -> &mut PersistentIncomingServiceState {
        match self
            .incoming_services
            .iter_mut()
            .position(|service| service.invitation_id == id)
        {
            // we have to use index, see https://github.com/rust-lang/rust/issues/21906
            Some(index) => &mut self.incoming_services[index],
            None => {
                self.incoming_services.push(PersistentIncomingServiceState {
                    invitation_id: id.to_string(),
                    enabled: true,
                    name: None,
                });
                self.incoming_services.last_mut().unwrap()
            }
        }
    }
}

impl AppState {
    pub async fn load_services_from_invites(
        &self,
        accepted_invitations: Vec<InvitationWithAccess>,
    ) {
        let incoming_services_arc = self.incoming_services();
        let mut guard = incoming_services_arc.write().await;
        for invite in accepted_invitations {
            // during the synchronization we only need to add new ones
            if guard.find_by_id(&invite.invitation.id).is_some() {
                continue;
            }

            if let Some(service_access_details) = invite.service_access_details {
                let mut enabled = true;
                let mut name = None;

                if let Some(state) = self
                    .model(|m| {
                        m.incoming_services
                            .iter()
                            .find(|s| s.invitation_id == invite.invitation.id)
                            .cloned()
                    })
                    .await
                {
                    enabled = state.enabled;
                    name = state.name.clone();
                }

                let name = if let Some(name) = name {
                    name
                } else {
                    match service_access_details.service_name() {
                        Ok(name) => name,
                        Err(err) => {
                            warn!(%err, "Failed to get service name from access details");
                            continue;
                        }
                    }
                };

                guard.services.push(IncomingService::new(
                    invite.invitation.id,
                    name,
                    None,
                    enabled,
                    service_access_details,
                ));
            } else {
                warn!(
                    "No service access details found for accepted invitations {}",
                    invite.invitation.id
                );
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct IncomingService {
    id: String,
    name: String,
    port: Option<u16>,
    enabled: bool,
    access_details: ServiceAccessDetails,
}

impl IncomingService {
    pub fn new(
        id: String,
        name: String,
        port: Option<u16>,
        enabled: bool,
        access_details: ServiceAccessDetails,
    ) -> Self {
        Self {
            id,
            name,
            port,
            enabled,
            access_details,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn original_name(&self) -> Result<String, ApiError> {
        self.access_details.service_name()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn address(&self) -> Option<SocketAddr> {
        self.port
            .map(|port| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port))
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_port(&mut self, port: Option<u16>) {
        self.port = port;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
    pub fn disable(&mut self) {
        self.enabled = false;
        self.port = None;
    }

    /// Returns the enrollment ticket, to avoid conflicts with 'default' name
    /// the project name is re-set to the project id
    pub fn enrollment_ticket(&self) -> ockam::Result<EnrollmentTicket> {
        let mut enrollment_ticket = self.access_details.enrollment_ticket()?;
        if let Some(project) = enrollment_ticket.project.as_mut() {
            project.name = project.id.clone();
        }
        Ok(enrollment_ticket)
    }

    pub fn relay_name(&self) -> String {
        let bare_relay_name = self.access_details.shared_node_identity.to_string();
        format!("forward_to_{bare_relay_name}")
    }

    /// Returns the name of the inlet within the node, for now it's a constant
    pub fn inlet_name(&self) -> &str {
        "app-inlet"
    }

    pub fn service_route(&self) -> ockam::Result<String> {
        if let Some(project) = self.enrollment_ticket()?.project.as_ref() {
            let project_id = &project.id;
            let relay_name = self.relay_name();
            let service_name = match self.original_name() {
                Ok(name) => name,
                Err(_) => {
                    warn!("Failed to get service name from access details");
                    return Err(ApiError::core(
                        "Failed to get service name from access details",
                    ));
                }
            };
            Ok(format!(
                "/project/{project_id}/service/{relay_name}/secure/api/service/{service_name}"
            ))
        } else {
            Err(ApiError::core("No project id found in enrollment ticket"))
        }
    }

    pub fn local_node_name(&self) -> ockam::Result<String> {
        if let Some(project) = self.enrollment_ticket()?.project.as_ref() {
            let project_id = &project.id;
            let id = &self.id;
            Ok(format!("ockam_app_{project_id}_{id}"))
        } else {
            Err(ApiError::core("No project id found in enrollment ticket"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::incoming_services::PersistentIncomingServiceState;
    use crate::state::AppState;
    use ockam::identity::{Identifier, OneTimeCode};
    use ockam::Context;
    use ockam_api::cloud::share::{
        InvitationWithAccess, ReceivedInvitation, RoleInShare, ServiceAccessDetails, ShareScope,
    };
    use ockam_api::config::lookup::ProjectLookup;
    use ockam_api::identity::EnrollmentTicket;
    use std::str::FromStr;

    #[ockam::test(crate = "ockam")]
    async fn test_inlet_data_from_invitation(ctx: &mut Context) -> ockam::Result<()> {
        let mut invitation = InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner_email".to_string(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
                ignored: false,
            },
            service_access_details: None,
        };

        // Skipping an invitation without service access details
        let app_state = AppState::test(ctx).await.unwrap();

        app_state
            .load_services_from_invites(vec![invitation.clone()])
            .await;

        let services = app_state.incoming_services().read().await.services.clone();
        assert!(services.is_empty(), "No services should be loaded");

        invitation.service_access_details = Some(ServiceAccessDetails {
            project_identity: "I1234561234561234561234561234561234561234"
                .try_into()
                .unwrap(),
            project_route: "mock_project_route".to_string(),
            project_authority_identity: "Iabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .try_into()
                .unwrap(),
            project_authority_route: "project_authority_route".to_string(),
            shared_node_identity: "I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab"
                .try_into()
                .unwrap(),
            shared_node_route: "remote_service_name".to_string(),
            enrollment_ticket: EnrollmentTicket::new(
                OneTimeCode::new(),
                Some(ProjectLookup {
                    node_route: None,
                    id: "project_id".to_string(),
                    name: "project_name".to_string(),
                    identity_id: Some(
                        Identifier::from_str("I1234561234561234561234561234561234561234").unwrap(),
                    ),
                    authority: None,
                    okta: None,
                }),
                None,
            )
            .hex_encoded()
            .unwrap(),
        });

        app_state
            .load_services_from_invites(vec![invitation.clone()])
            .await;
        let services = app_state.incoming_services().read().await.services.clone();
        assert_eq!(1, services.len(), "One service should be loaded");

        let service = &services[0];
        assert_eq!("invitation_id", service.id());
        assert_eq!(
            "I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab",
            service.access_details.shared_node_identity.to_string()
        );
        assert_eq!("remote_service_name", service.name());
        assert!(service.enabled());
        assert!(service.port().is_none());
        assert_eq!(
            "project_id",
            service.enrollment_ticket().unwrap().project.unwrap().name,
            "project name should be overwritten with project id"
        );
        assert_eq!(
            "forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab",
            service.relay_name()
        );
        assert_eq!("/project/project_id/service/forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab/secure/api/service/remote_service_name", service.service_route().unwrap());
        assert_eq!(
            "ockam_app_project_id_invitation_id",
            service.local_node_name().unwrap()
        );

        let second_invitation = InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "second_invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner_email".to_string(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
                ignored: false,
            },
            service_access_details: invitation.service_access_details.clone(),
        };

        // let's load another invitation, but but a persistent state for it already exists
        app_state
            .model_mut(|m| {
                m.incoming_services.push(PersistentIncomingServiceState {
                    invitation_id: "second_invitation_id".to_string(),
                    enabled: false,
                    name: Some("custom_user_name".to_string()),
                })
            })
            .await
            .unwrap();

        app_state
            .load_services_from_invites(vec![invitation.clone(), second_invitation.clone()])
            .await;
        let services = app_state.incoming_services().read().await.services.clone();
        assert_eq!(2, services.len(), "Two services should be loaded");

        let service = &services[1];
        assert_eq!("second_invitation_id", service.id());
        assert!(!service.enabled());
        assert_eq!("custom_user_name", service.name());
        assert_eq!("remote_service_name", service.original_name().unwrap());
        assert_eq!("/project/project_id/service/forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab/secure/api/service/remote_service_name", service.service_route().unwrap());
        ctx.stop().await
    }
}
