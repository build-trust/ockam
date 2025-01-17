use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::share::{
    AcceptInvitation, AcceptedInvitation, CreateInvitation, CreateServiceInvitation,
    InvitationList, InvitationListKind, InvitationWithAccess, ListInvitations, RoleInShare,
    SentInvitation, ShareScope,
};
use crate::orchestrator::{ControllerClient, HasSecureClient};
use miette::IntoDiagnostic;
use ockam::identity::Identifier;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

const API_SERVICE: &str = "users";

#[async_trait]
pub trait Invitations {
    #[allow(clippy::too_many_arguments)]
    async fn create_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        grant_role: RoleInShare,
        recipient_email: EmailAddress,
        remaining_uses: Option<usize>,
        scope: ShareScope,
        target_id: String,
    ) -> miette::Result<SentInvitation>;

    #[allow(clippy::too_many_arguments)]
    async fn create_service_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        project_id: String,
        recipient_email: EmailAddress,
        project_identity: Identifier,
        project_route: String,
        project_authority_identity: Identifier,
        project_authority_route: String,
        shared_node_identity: Identifier,
        shared_node_route: String,
        enrollment_ticket: String,
    ) -> miette::Result<SentInvitation>;

    async fn accept_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<AcceptedInvitation>;

    async fn show_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<InvitationWithAccess>;

    async fn list_invitations(
        &self,
        ctx: &Context,
        kind: InvitationListKind,
    ) -> miette::Result<InvitationList>;

    async fn ignore_invitation(&self, ctx: &Context, invitation_id: String) -> miette::Result<()>;
}

#[async_trait]
impl Invitations for ControllerClient {
    async fn create_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        grant_role: RoleInShare,
        recipient_email: EmailAddress,
        remaining_uses: Option<usize>,
        scope: ShareScope,
        target_id: String,
    ) -> miette::Result<SentInvitation> {
        trace!(%scope, target_id = %target_id, "creating invitation");
        let req_body = CreateInvitation {
            expires_at,
            grant_role,
            recipient_email,
            remaining_uses,
            scope,
            target_id,
        };
        let req = Request::post("/v0/invites").body(req_body);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("create invitation")
    }

    async fn create_service_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        project_id: String,
        recipient_email: EmailAddress,
        project_identity: Identifier,
        project_route: String,
        project_authority_identity: Identifier,
        project_authority_route: String,
        shared_node_identity: Identifier,
        shared_node_route: String,
        enrollment_ticket: String,
    ) -> miette::Result<SentInvitation> {
        trace!(project_id = %project_id, "creating service invitation");
        let req_body = CreateServiceInvitation {
            expires_at,
            project_id,
            recipient_email,
            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,
            enrollment_ticket,
        };
        let req = Request::post("/v0/invites/service").body(req_body);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("create service invitation")
    }

    async fn accept_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<AcceptedInvitation> {
        let req = Request::post("/v0/redeem_invite").body(AcceptInvitation { id: invitation_id });
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("redeem invitation")
    }

    async fn show_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<InvitationWithAccess> {
        trace!(?invitation_id, "showing invitation");
        let req = Request::get(format!("/v0/invites/{invitation_id}"));
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("get invitation")
    }

    async fn list_invitations(
        &self,
        ctx: &Context,
        kind: InvitationListKind,
    ) -> miette::Result<InvitationList> {
        debug!(?kind, "Sending request to list shares");
        let req = Request::get("/v0/invites").body(ListInvitations { kind });
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("list invitations")
    }

    async fn ignore_invitation(&self, ctx: &Context, invitation_id: String) -> miette::Result<()> {
        debug!(?invitation_id, "sending request to ignore invitation");
        let req = Request::post(format!("/v0/invites/{invitation_id}/ignore"));
        self.get_secure_client()
            .tell(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("ignore invitation")
    }
}
