use crate::cloud::share::{
    AcceptInvitation, AcceptedInvitation, CreateInvitation, CreateServiceInvitation,
    InvitationList, InvitationListKind, InvitationWithAccess, ListInvitations, RoleInShare,
    SentInvitation, ShareScope,
};
use crate::cloud::Controller;
use miette::IntoDiagnostic;
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
        recipient_email: String,
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
        recipient_email: String,
        project_identity: String,
        project_route: String,
        project_authority_identity: String,
        project_authority_route: String,
        shared_node_identity: String,
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
}

#[async_trait]
impl Invitations for Controller {
    async fn create_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        grant_role: RoleInShare,
        recipient_email: String,
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
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn create_service_invitation(
        &self,
        ctx: &Context,
        expires_at: Option<String>,
        project_id: String,
        recipient_email: String,
        project_identity: String,
        project_route: String,
        project_authority_identity: String,
        project_authority_route: String,
        shared_node_identity: String,
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
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn accept_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<AcceptedInvitation> {
        let req = Request::post("/v0/redeem_invite").body(AcceptInvitation { id: invitation_id });
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn show_invitation(
        &self,
        ctx: &Context,
        invitation_id: String,
    ) -> miette::Result<InvitationWithAccess> {
        trace!(?invitation_id, "showing invitation");
        let req = Request::get(format!("/v0/invites/{invitation_id}"));
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_invitations(
        &self,
        ctx: &Context,
        kind: InvitationListKind,
    ) -> miette::Result<InvitationList> {
        debug!(kink = ?kind, "Sending request to list shares");
        let req = Request::get("/v0/invites").body(ListInvitations { kind });
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
