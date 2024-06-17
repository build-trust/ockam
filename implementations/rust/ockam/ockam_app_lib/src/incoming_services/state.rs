use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use tracing::warn;

use ockam::identity::Identifier;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::email_address::EmailAddress;
use ockam_api::cloud::share::InvitationWithAccess;

use crate::state::{AppState, ModelState};

/// A Socket port number
pub type Port = u16;

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Deserialize, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PersistentIncomingService {
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

    pub(crate) fn remove_by_id(&mut self, id: &str) {
        if let Some(index) = self.services.iter().position(|s| s.id == id) {
            self.services.remove(index);
        }
    }

    pub(crate) fn find_mut_by_id(&mut self, id: &str) -> Option<&mut IncomingService> {
        self.services.iter_mut().find(|s| s.id == id)
    }
}

impl ModelState {
    pub(crate) fn upsert_incoming_service(&mut self, id: &str) -> &mut PersistentIncomingService {
        match self
            .incoming_services
            .iter_mut()
            .position(|service| service.invitation_id == id)
        {
            // we have to use index, see https://github.com/rust-lang/rust/issues/21906
            Some(index) => &mut self.incoming_services[index],
            None => {
                self.incoming_services.push(PersistentIncomingService {
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
    pub async fn load_services_from_invitations(
        &self,
        accepted_invitations: Vec<InvitationWithAccess>,
    ) {
        let incoming_services_arc = self.incoming_services();
        let mut guard = incoming_services_arc.write().await;

        // first let's remove services that are not in the list of accepted invitations
        // and mark the as removed, so relative resource will be freed before removing
        // them from the list
        for service in guard.services.iter_mut() {
            if !accepted_invitations
                .iter()
                .any(|invite| invite.invitation.id == service.id)
            {
                service.mark_as_removed();
            }
        }

        for invite in accepted_invitations {
            // during the synchronization we only need to add new ones
            // assuming the invitation won't change
            if guard.find_by_id(&invite.invitation.id).is_some() {
                continue;
            }

            let service_access_details = match invite.service_access_details {
                None => {
                    warn!(
                        "No service access details found for accepted invitations {}",
                        invite.invitation.id
                    );
                    continue;
                }
                Some(service_access_details) => service_access_details,
            };

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

            let original_name = match service_access_details.service_name() {
                Ok(name) => name,
                Err(err) => {
                    warn!(%err, "Failed to get service name from access details");
                    continue;
                }
            };

            let mut ticket = match service_access_details.enrollment_ticket() {
                Ok(ticket) => ticket,
                Err(err) => {
                    warn!(%err, "Failed to parse enrollment ticket from access details");
                    continue;
                }
            };

            let project = if let Some(project) = ticket.project.as_mut() {
                // to avoid conflicts with 'default' name
                project.name = project.id.clone();
                project
            } else {
                warn!("No project found in enrollment ticket");
                continue;
            };

            guard.services.push(IncomingService::new(
                invite.invitation.id,
                invite.invitation.owner_email,
                name.unwrap_or_else(|| original_name.clone()),
                None,
                enabled,
                project.id.clone(),
                service_access_details.shared_node_identity,
                original_name,
                ticket,
            ));
        }
    }
}

#[derive(Clone, Debug)]
/// This structure represent the live information about an incoming service
/// This status is a reflection of three source of truth:
///     - an accepted invitation, which contains the service access details
///       as well as the id and the default name
///     - live inlet status: which contains the port number (when available)
///     - persistent state: which contains the user-defined name and the enabled status
pub struct IncomingService {
    // it's assumed the id is also the accepted invitation id
    id: String,
    // the email of the inviter
    email: EmailAddress,
    // user-defined name, by default it's the same as the original name
    name: String,
    // this field contains the current port number
    // it also reflects if the inlet is connected with the destination node
    port: Option<Port>,
    // true when connected to the destination node
    connected: bool,
    // whether the service should be enabled or not, this is the driver for the inlet
    // and may not reflect the current status
    enabled: bool,
    // all remaining fields were extracted from the access details
    project_id: String,
    // the identity identifier of the destination node, used to reconstruct the full route
    shared_node_identifier: Identifier,
    // this is used as the outlet service name, and it's needed
    // to reconstruct the full route
    original_name: String,
    // this enrollment ticket is modified to avoid conflicts with 'default' name
    // the name of the project is re-set to 'project_id'
    enrollment_ticket: EnrollmentTicket,
    // When the invitation is removed, the service is marked as removed
    // to clean up the resources before removing the service from the list
    removed: bool,
}

impl IncomingService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        email: EmailAddress,
        name: String,
        port: Option<Port>,
        enabled: bool,
        project_id: String,
        shared_node_identifier: Identifier,
        original_name: String,
        enrollment_ticket: EnrollmentTicket,
    ) -> Self {
        Self {
            id,
            email,
            name,
            port,
            enabled,
            project_id,
            shared_node_identifier,
            original_name,
            enrollment_ticket,
            connected: false,
            removed: false,
        }
    }

    /// This is the id of the service as well as of the relative invitation
    pub fn id(&self) -> &str {
        &self.id
    }

    /// This is the email of the inviter
    pub fn email(&self) -> &EmailAddress {
        &self.email
    }

    /// This is the user-defined name of the service
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The port number of the inlet, if service is connected to the destination node
    pub fn port(&self) -> Option<Port> {
        self.port
    }

    /// Returns true if the inlet is connected to the destination node
    pub fn is_connected(&self) -> bool {
        self.connected && self.port.is_some() && self.enabled
    }

    /// The address of the inlet, if service is connected to the destination node
    pub fn address(&self) -> Option<SocketAddr> {
        self.port
            .map(|port| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port))
    }

    /// Whether the service is enabled or not, this may not reflect the current state
    pub fn enabled(&self) -> bool {
        if self.removed {
            false
        } else {
            self.enabled
        }
    }

    pub fn set_port(&mut self, port: Port) {
        self.port = Some(port);
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// True when the service is marked as removed
    pub fn removed(&self) -> bool {
        self.removed
    }

    /// Mark the service as removed, following [`enabled()`] will return false.
    pub fn mark_as_removed(&mut self) {
        self.removed = true;
    }

    /// Returns the enrollment ticket, to avoid conflicts with 'default' name
    /// the project name is re-set to the project id
    pub fn enrollment_ticket(&self) -> &EnrollmentTicket {
        &self.enrollment_ticket
    }

    /// The relay name within the target project
    pub fn relay_name(&self) -> String {
        let bare_relay_name = self.shared_node_identifier.to_string();
        format!("forward_to_{bare_relay_name}")
    }

    /// Returns the full route to the outlet service
    pub fn service_route(&self, project_name: Option<&str>) -> String {
        // when dealing with accepted invitations, the project name is the project id
        let project_name = project_name.unwrap_or(&self.project_id);
        let relay_name = self.relay_name();
        let service_name = &self.original_name;
        format!("/project/{project_name}/service/{relay_name}/secure/api/service/{service_name}")
    }

    /// The name of the node that hosts the inlet
    pub fn local_node_name(&self) -> String {
        let project_id = &self.project_id;
        let id = &self.id;
        format!("ockam_app_{project_id}_{id}")
    }

    /// Returns the name of the inlet within the node, for now it's a constant
    pub fn inlet_name(&self) -> &str {
        "app-inlet"
    }
}

#[cfg(test)]
mod tests {
    use ockam::Context;
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::cli_state::enrollments::EnrollmentTicket;
    use ockam_api::cli_state::CliState;
    use ockam_api::cloud::project::models::ProjectModel;
    use ockam_api::cloud::share::{
        InvitationWithAccess, ReceivedInvitation, RoleInShare, ServiceAccessDetails, ShareScope,
    };

    use crate::incoming_services::PersistentIncomingService;
    use crate::state::AppState;

    fn create_invitation_with(
        service_access_details: Option<ServiceAccessDetails>,
    ) -> InvitationWithAccess {
        InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner@email".try_into().unwrap(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
                ignored: false,
            },
            service_access_details,
        }
    }

    fn create_service_access() -> ServiceAccessDetails {
        ServiceAccessDetails {
            project_identity: "I1234561234561234561234561234561234561234a1b2c3d4e5f6a6b5c4d3e2f1"
                .try_into()
                .unwrap(),
            project_route: "mock_project_route".to_string(),
            project_authority_identity:
            "Iabcdefabcdefabcdefabcdefabcdefabcdefabcda1b2c3d4e5f6a6b5c4d3e2f1"
                .try_into()
                .unwrap(),
            project_authority_route: "project_authority_route".to_string(),
            shared_node_identity:
            "I12ab34cd56ef12ab34cd56ef12ab34cd56ef12aba1b2c3d4e5f6a6b5c4d3e2f1"
                .try_into()
                .unwrap(),
            shared_node_route: "remote_service_name".to_string(),
            enrollment_ticket: EnrollmentTicket::new(
                OneTimeCode::new(),
                Some(ProjectModel {
                    id: "project_id".to_string(),
                    name: "project_name".to_string(),
                    space_name: "space_name".to_string(),
                    access_route: "route".to_string(),
                    users: vec![],
                    space_id: "space_id".to_string(),
                    identity: None,
                    project_change_history: None,
                    authority_access_route: Some("/project/authority_route".to_string()),
                    authority_identity: Some("81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805".to_string()),
                    okta_config: None,
                    kafka_config: None,
                    version: None,
                    running: None,
                    operation_id: None,
                    user_roles: vec![],
                }),
            )
                .hex_encoded()
                .unwrap(),
        }
    }

    #[ockam::test(crate = "ockam")]
    async fn test_inlet_data_from_invitation(context: &mut Context) -> ockam::Result<()> {
        // in this test we want to validate data loading from the accepted invitation
        // as well as using the related persistent data
        let app_state = AppState::test(context, CliState::test().await?).await;

        let mut invitation = create_invitation_with(None);

        // invitation without service access details should be skipped
        app_state
            .load_services_from_invitations(vec![invitation.clone()])
            .await;

        let services = app_state.incoming_services().read().await.services.clone();
        assert!(services.is_empty(), "No services should be loaded");

        invitation.service_access_details = Some(create_service_access());

        // then we load the invitation with service access details
        app_state
            .load_services_from_invitations(vec![invitation.clone()])
            .await;
        let services = app_state.incoming_services().read().await.services.clone();
        assert_eq!(1, services.len(), "One service should be loaded");

        let service = &services[0];
        assert_eq!("invitation_id", service.id());
        assert_eq!("remote_service_name", service.name());
        assert!(service.enabled());
        assert!(service.port().is_none());
        assert_eq!(
            "project_id",
            service.enrollment_ticket().project.as_ref().unwrap().name,
            "project name should be overwritten with project id"
        );
        assert_eq!(
            "forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12aba1b2c3d4e5f6a6b5c4d3e2f1",
            service.relay_name()
        );
        assert_eq!("/project/project_id/service/forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12aba1b2c3d4e5f6a6b5c4d3e2f1/secure/api/service/remote_service_name", service.service_route(None));
        assert_eq!("/project/custom-project-name/service/forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12aba1b2c3d4e5f6a6b5c4d3e2f1/secure/api/service/remote_service_name", service.service_route(Some("custom-project-name")));
        assert_eq!(
            "ockam_app_project_id_invitation_id",
            service.local_node_name()
        );

        let second_invitation = InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "second_invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner@email".try_into().unwrap(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
                ignored: false,
            },
            service_access_details: invitation.service_access_details.clone(),
        };

        // let's load another invitation, but persistent state for it already exists
        app_state
            .model_mut(|m| {
                m.incoming_services.push(PersistentIncomingService {
                    invitation_id: "second_invitation_id".to_string(),
                    enabled: false,
                    name: Some("custom_user_name".to_string()),
                })
            })
            .await
            .unwrap();

        app_state
            .load_services_from_invitations(vec![invitation.clone(), second_invitation.clone()])
            .await;
        let services = app_state.incoming_services().read().await.services.clone();
        assert_eq!(2, services.len(), "Two services should be loaded");

        let service = &services[1];
        assert_eq!("second_invitation_id", service.id());
        assert!(!service.enabled());
        assert_eq!("custom_user_name", service.name());
        assert_eq!("/project/project_id/service/forward_to_I12ab34cd56ef12ab34cd56ef12ab34cd56ef12aba1b2c3d4e5f6a6b5c4d3e2f1/secure/api/service/remote_service_name", service.service_route(None));

        context.stop().await
    }
}
