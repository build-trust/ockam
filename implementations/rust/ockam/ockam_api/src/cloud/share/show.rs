use super::InvitationWithAccess;

mod node {
    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::Context;

    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const API_SERVICE: &str = "users";

    impl NodeManager {
        pub async fn show_invitation(
            &self,
            ctx: &Context,
            invitation_id: &str,
            route: &MultiAddr,
        ) -> Result<InvitationWithAccess> {
            Response::parse_response_body(
                self.show_invitation_response(ctx, invitation_id, CloudRequestWrapper::bare(route))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn show_invitation_response(
            &self,
            ctx: &Context,
            invitation_id: &str,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "show_invitation";
            trace!(?invitation_id, "showing invitation");
            let req_builder = Request::get(format!("/v0/invites/{invitation_id}"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
    }

    impl NodeManagerWorker {
        pub async fn show_invitation(
            &self,
            ctx: &Context,
            invitation_id: &str,
            route: &MultiAddr,
        ) -> Result<InvitationWithAccess> {
            let node_manager = self.inner().read().await;
            node_manager
                .show_invitation(ctx, invitation_id, route)
                .await
        }

        pub(crate) async fn show_invitation_response(
            &self,
            ctx: &Context,
            invitation_id: &str,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .show_invitation_response(ctx, invitation_id, req_wrapper)
                .await
        }
    }
}
