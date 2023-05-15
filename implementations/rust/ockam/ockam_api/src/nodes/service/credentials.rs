use crate::cli_state::traits::StateDirTrait;
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::service::map_multiaddr_err;
use either::Either;
use minicbor::Decoder;
use ockam::identity::Credential;
use ockam::Result;
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_identity::IdentitiesVault;
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use std::str::FromStr;
use std::sync::Arc;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) async fn get_credential(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Either<ResponseBuilder<Error<'_>>, ResponseBuilder<Credential>>> {
        let node_manager = self.node_manager.write().await;
        let request: GetCredentialRequest = dec.decode()?;

        let identity = if let Some(identity) = &request.identity_name {
            let idt_state = node_manager.cli_state.identities.get(identity)?;
            match idt_state.get(node_manager.identities_vault()).await {
                Ok(idt) => Arc::new(idt),
                Err(_) => {
                    let default_vault = &node_manager.cli_state.vaults.default()?.get().await?;
                    let vault: Arc<dyn IdentitiesVault> = Arc::new(default_vault.clone());
                    Arc::new(idt_state.get(vault).await?)
                }
            }
        } else {
            Arc::new(node_manager.identity())
        };

        if let Ok(c) = node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &identity.identifier())
            .await
        {
            Ok(Either::Right(Response::ok(req.id()).body(c)))
        } else {
            let err = Error::default().with_message("error getting credential");
            Ok(Either::Left(Response::internal_error(req.id()).body(err)))
        }
    }

    pub(super) async fn present_credential(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder> {
        let node_manager = self.node_manager.write().await;
        let request: PresentCredentialRequest = dec.decode()?;

        // TODO: Replace with self.connect?
        let route = MultiAddr::from_str(&request.route).map_err(map_multiaddr_err)?;
        let route = match local_multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::generic("invalid credentials service route")),
        };

        let credential = node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &node_manager.identity.identifier())
            .await?;

        if request.oneway {
            node_manager
                .credentials_service()
                .present_credential(
                    ctx,
                    route,
                    credential,
                    MessageSendReceiveOptions::new().with_flow_control(&node_manager.flow_controls),
                )
                .await?;
        } else {
            node_manager
                .credentials_service()
                .present_credential_mutual(
                    ctx,
                    route,
                    &[node_manager.trust_context()?.authority()?.identity()],
                    credential,
                    MessageSendReceiveOptions::new().with_flow_control(&node_manager.flow_controls),
                )
                .await?;
        }

        let response = Response::ok(req.id());
        Ok(response)
    }
}
