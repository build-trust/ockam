use crate::authenticator::direct::Client;
use crate::cli_state::CliState;
use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::service::map_multiaddr_err;
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use either::Either;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::{route, AsyncTryClone};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::credential::Credential;
use ockam_identity::{Identity, IdentityVault};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::str::FromStr;

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn get_credential_impl<V: IdentityVault, S: AuthenticatedStorage>(
        &mut self,
        identity: &Identity<V, S>,
        overwrite: bool,
    ) -> Result<()> {
        debug!("Credential check: looking for identity");

        if identity.credential().await.is_some() && !overwrite {
            return Err(ApiError::generic("credential already exists"));
        }

        debug!("Credential check: looking for authorities...");
        let authorities = self.authorities()?;

        // Take first authority
        let authority = authorities
            .as_ref()
            .first()
            .ok_or_else(|| ApiError::generic("No known Authority"))?;

        debug!("Getting credential from : {}", authority.addr);

        let allowed = vec![authority.identity.identifier().clone()];

        let route = match multiaddr_to_route(&authority.addr) {
            Some(route) => route,
            None => {
                error!("INVALID ROUTE");
                return Err(ApiError::generic("invalid authority route"));
            }
        };

        debug!("Create secure channel to project authority");
        let sc = self
            .create_secure_channel_internal(identity, route, Some(allowed), None)
            .await?;
        debug!("Created secure channel to project authority");

        let enrollment_token = self.token.take();

        // Borrow checker issues...
        let authorities = self.authorities()?;

        let mut client =
            Client::new(route![sc, DefaultAddress::AUTHENTICATOR], identity.ctx()).await?;
        let credential = if let Some(code) = enrollment_token {
            client.credential_with(&code).await?
        } else {
            client.credential().await?
        };
        debug!("Got credential");

        identity
            .verify_self_credential(&credential, authorities.public_identities().iter())
            .await?;
        debug!("Verified self credential");

        identity.set_credential(credential.to_owned()).await;

        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) async fn get_credential(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Either<ResponseBuilder<Error<'_>>, ResponseBuilder<Credential>>> {
        let mut node_manager = self.node_manager.write().await;
        let request: GetCredentialRequest = dec.decode()?;

        let identity = if let Some(identity) = &request.identity_name {
            let state = CliState::new()?;
            let idt_config = state.identities.get(identity)?.config;
            match idt_config.get(ctx, node_manager.vault()?).await {
                Ok(idt) => idt,
                Err(_) => {
                    let default_vault = &state.vaults.default()?.config.get().await?;
                    idt_config.get(ctx, default_vault).await?
                }
            }
        } else {
            node_manager.identity()?.async_try_clone().await?
        };

        node_manager
            .get_credential_impl(&identity, request.is_overwrite())
            .await?;

        if let Some(c) = identity.credential().await {
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
    ) -> Result<ResponseBuilder> {
        let node_manager = self.node_manager.read().await;
        let request: PresentCredentialRequest = dec.decode()?;

        let route = MultiAddr::from_str(&request.route).map_err(map_multiaddr_err)?;
        let route = match multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::generic("invalid credentials service route")),
        };

        let identity = node_manager.identity()?;

        if request.oneway {
            identity.present_credential(route).await?;
        } else {
            identity
                .present_credential_mutual(
                    route,
                    &node_manager.authorities()?.public_identities(),
                    &node_manager.attributes_storage,
                )
                .await?;
        }

        let response = Response::ok(req.id());
        Ok(response)
    }
}
