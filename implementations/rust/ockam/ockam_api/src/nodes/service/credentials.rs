use crate::authenticator::direct::{CredentialIssuerClient, RpcClient};
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::service::map_multiaddr_err;
use crate::nodes::NodeManager;
use crate::{create_tcp_session, DefaultAddress};
use either::Either;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::route;
use ockam_identity::credential::Credential;
use ockam_identity::{Identity, IdentityVault};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use std::str::FromStr;

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn get_credential_impl(
        &mut self,
        identity: &Identity,
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

        let authority_tcp_session = match create_tcp_session(
            &authority.addr,
            &self.tcp_transport,
            &self.message_flow_sessions,
        )
        .await
        {
            Some(authority_tcp_session) => authority_tcp_session,
            None => {
                error!("INVALID ROUTE");
                return Err(ApiError::generic("invalid authority route"));
            }
        };

        debug!("Create secure channel to project authority");
        let (sc, _sc_session_id) = self
            .create_secure_channel_internal(
                identity,
                authority_tcp_session.route,
                Some(allowed),
                None,
            )
            .await?;
        debug!("Created secure channel to project authority");

        // Borrow checker issues...
        let authorities = self.authorities()?;

        let client = CredentialIssuerClient::new(
            RpcClient::new(
                route![sc, DefaultAddress::CREDENTIAL_ISSUER],
                identity.ctx(),
            )
            .await?
            .with_sessions(&self.message_flow_sessions),
        );
        let credential = client.credential().await?;
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

        // TODO: Replace with self.connect?
        let route = MultiAddr::from_str(&request.route).map_err(map_multiaddr_err)?;
        let route = match local_multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::generic("invalid credentials service route")),
        };

        if request.oneway {
            node_manager
                .identity
                .present_credential(
                    route,
                    None,
                    MessageSendReceiveOptions::new()
                        .with_session(&node_manager.message_flow_sessions),
                )
                .await?;
        } else {
            node_manager
                .identity
                .present_credential_mutual(
                    route,
                    &node_manager.authorities()?.public_identities(),
                    node_manager.attributes_storage.clone(),
                    None,
                    MessageSendReceiveOptions::new()
                        .with_session(&node_manager.message_flow_sessions),
                )
                .await?;
        }

        let response = Response::ok(req.id());
        Ok(response)
    }
}
