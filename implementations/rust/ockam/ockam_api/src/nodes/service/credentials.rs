use crate::authenticator::direct::types::OneTimeCode;
use crate::authenticator::direct::Client;
use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::service::map_multiaddr_err;
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::{route, AsyncTryClone};
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn get_credential_impl(
        &mut self,
        overwrite: bool,
        code: Option<&OneTimeCode>,
    ) -> Result<()> {
        debug!("Credential check: looking for identity");
        let identity = self.identity()?.async_try_clone().await?;

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
            .create_secure_channel_internal(&identity, route, Some(allowed), None)
            .await?;
        debug!("Created secure channel to project authority");

        // Borrow checker issues...
        let authorities = self.authorities()?;

        let mut client =
            Client::new(route![sc, DefaultAddress::AUTHENTICATOR], identity.ctx()).await?;

        let credential = if let Some(c) = code {
            client.credential_with(c).await?
        } else {
            client.credential().await?
        };

        debug!("Got credential");

        identity
            .verify_self_credential(&credential, authorities.public_identities().iter())
            .await?;
        debug!("Verified self credential");

        identity.set_credential(Some(credential.to_owned())).await;

        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) async fn get_credential(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let request: GetCredentialRequest = dec.decode()?;

        node_manager
            .get_credential_impl(request.is_overwrite(), request.code())
            .await?;

        let response = Response::ok(req.id());
        Ok(response)
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
                    &node_manager.authenticated_storage,
                )
                .await?;
        }

        let response = Response::ok(req.id());
        Ok(response)
    }
}
