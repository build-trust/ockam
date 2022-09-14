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

impl NodeManager {
    pub(super) async fn get_credential_impl(&mut self, overwrite: bool) -> Result<()> {
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
            .create_secure_channel_internal(&identity, route, Some(allowed))
            .await?;
        debug!("Created secure channel to project authority");

        // Borrow checker issues...
        let authorities = self.authorities()?;

        let mut client =
            Client::new(route![sc, DefaultAddress::AUTHENTICATOR], identity.ctx()).await?;
        let credential = client.credential().await?;
        debug!("Got credential");

        identity
            .verify_self_credential(&credential, authorities.public_identities().iter())
            .await?;
        debug!("Verified self credential");

        identity.set_credential(Some(credential.to_owned())).await;

        Ok(())
    }

    pub(super) async fn get_credential(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let request: GetCredentialRequest = dec.decode()?;

        self.get_credential_impl(request.overwrite).await?;

        let response = Response::ok(req.id());
        Ok(response)
    }

    pub(super) async fn present_credential(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let request: PresentCredentialRequest = dec.decode()?;

        let route = MultiAddr::from_str(&request.route).map_err(map_multiaddr_err)?;
        let route = match multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::generic("invalid credentials service route")),
        };

        let identity = self.identity()?;

        if request.oneway {
            identity.present_credential(route).await?;
        } else {
            identity
                .present_credential_mutual(
                    route,
                    &self.authorities()?.public_identities(),
                    &self.authenticated_storage,
                )
                .await?;
        }

        let response = Response::ok(req.id());
        Ok(response)
    }
}
