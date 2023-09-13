use super::InvitationWithAccess;

mod node {
    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const API_SERVICE: &str = "users";

    impl NodeManager {
        pub async fn show_invitation(
            &self,
            ctx: &Context,
            invitation_id: &str,
        ) -> Result<InvitationWithAccess> {
            Response::parse_response_body(
                self.show_invitation_response(ctx, invitation_id)
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn show_invitation_response(
            &self,
            ctx: &Context,
            invitation_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(?invitation_id, "showing invitation");
            let req = Request::get(format!("/v0/invites/{invitation_id}"));
            self.make_controller_client()
                .await?
                .request(ctx, API_SERVICE, req)
                .await
        }
    }

    impl NodeManagerWorker {
        pub async fn show_invitation(
            &self,
            ctx: &Context,
            invitation_id: &str,
        ) -> Result<InvitationWithAccess> {
            let node_manager = self.inner().read().await;
            node_manager.show_invitation(ctx, invitation_id).await
        }

        pub(crate) async fn show_invitation_response(
            &self,
            ctx: &Context,
            invitation_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .show_invitation_response(ctx, invitation_id)
                .await
        }
    }
}
