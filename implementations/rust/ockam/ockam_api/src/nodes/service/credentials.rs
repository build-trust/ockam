use super::NodeManagerWorker;
use crate::authenticator::direct::RpcClient;
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::service::map_multiaddr_err;
use crate::nodes::NodeManager;
use either::Either;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::route;
use ockam_identity::credential::{Credential, CredentialExchangeMode};
use ockam_identity::credential_issuer::CredentialIssuerClient;
use ockam_identity::{Identity, IdentityVault};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::str::FromStr;

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
            match idt_state.get(ctx, node_manager.vault()?).await {
                Ok(idt) => Arc::new(idt),
                Err(_) => {
                    let default_vault = &node_manager.cli_state.vaults.default()?.get().await?;
                    let vault: Arc<dyn IdentityVault> = Arc::new(default_vault.clone());
                    Arc::new(idt_state.get(ctx, vault).await?)
                }
            }
        } else {
            node_manager.identity.clone()
        };

        if let Ok(c) = node_manager
            .trust_context()?
            .authority()?
            .credential(&identity)
            .await
        {
            Ok(Either::Right(Response::ok(req.id()).body(c)))
        } else {
            let err = Error::default().with_message("error getting credential");
            Ok(Either::Left(Response::internal_error(req.id()).body(err)))
        }
    }
}
