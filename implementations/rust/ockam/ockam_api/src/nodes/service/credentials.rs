use std::str::FromStr;

use either::Either;
use minicbor::Decoder;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::Result;
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cli_state::traits::StateDirTrait;
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) async fn get_credential(
        &mut self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Either<Response<Error>, Response<CredentialAndPurposeKey>>> {
        let node_manager = self.node_manager.write().await;
        let request: GetCredentialRequest = dec.decode()?;

        let identifier = if let Some(identity) = &request.identity_name {
            node_manager
                .cli_state
                .identities
                .get(identity)?
                .identifier()
        } else {
            node_manager.identifier()
        };

        match node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &identifier)
            .await
        {
            Ok(c) => Ok(Either::Right(Response::ok(req).body(c))),
            Err(e) => Ok(Either::Left(Response::internal_error(
                req,
                &format!(
                    "Error retrieving credential from authority for {}: {}",
                    identifier, e,
                ),
            ))),
        }
    }

    pub(super) async fn present_credential(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response, Response<Error>> {
        let node_manager = self.node_manager.write().await;
        let request: PresentCredentialRequest = dec.decode()?;

        // TODO: Replace with self.connect?
        let route = MultiAddr::from_str(&request.route).map_err(|_| {
            ApiError::core(format!(
                "Couldn't convert String to MultiAddr: {}",
                &request.route
            ))
        })?;
        let route = match local_multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::core("Invalid credentials service route").into()),
        };

        let credential = node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &node_manager.identifier())
            .await?;

        if request.oneway {
            node_manager
                .credentials_service()
                .present_credential(ctx, route, credential)
                .await?;
        } else {
            node_manager
                .credentials_service()
                .present_credential_mutual(
                    ctx,
                    route,
                    node_manager
                        .trust_context()?
                        .authorities()
                        .await?
                        .as_slice(),
                    credential,
                )
                .await?;
        }

        let response = Response::ok(req);
        Ok(response)
    }
}
