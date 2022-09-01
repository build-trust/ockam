use crate::authenticator::direct::Client;
use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::credentials::{
    GetCredentialRequest, PresentCredentialRequest, SetAuthorityRequest,
};
use crate::nodes::service::{map_multiaddr_err, Authorities, AuthorityInfo};
use crate::nodes::NodeManager;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_identity::{Identity, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;
use std::str::FromStr;

impl NodeManager {
    pub(super) async fn set_authorities_impl<'a>(&mut self, authorities: &[Vec<u8>]) -> Result<()> {
        let vault = self.vault()?;

        let mut authorities_vec = vec![];
        for authority in authorities {
            let identity = PublicIdentity::import(authority.as_ref(), vault).await?;
            authorities_vec.push(AuthorityInfo {
                identity,
                addr: None,
            });
        }
        self.authorities = Some(Authorities::new(authorities_vec));

        Ok(())
    }

    pub(super) async fn set_authorities<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let request: SetAuthorityRequest = dec.decode()?;

        self.set_authorities_impl(&request.authorities).await?;

        let response = Response::ok(req.id());
        Ok(response)
    }

    pub(super) async fn get_credential_impl(
        &self,
        identity: &Identity<Vault>,
        overwrite: bool,
        route: &MultiAddr,
    ) -> Result<()> {
        let authorities = self.authorities()?;

        if identity.credential().await.is_some() && !overwrite {
            return Err(ApiError::generic("credential already exists"));
        }

        let route = match multiaddr_to_route(route) {
            Some(route) => route,
            None => return Err(ApiError::generic("invalid authority route")),
        };

        let mut client = Client::new(route, identity.ctx()).await?;
        let credential = client.credential().await?;

        identity
            .verify_self_credential(&credential, authorities.public_identities().iter())
            .await?;

        identity.set_credential(Some(credential.to_owned())).await;

        Ok(())
    }

    pub(super) async fn get_credential(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let request: GetCredentialRequest = dec.decode()?;

        let identity = self.identity()?;

        let route = MultiAddr::from_str(request.route.as_ref()).map_err(map_multiaddr_err)?;
        self.get_credential_impl(identity, request.overwrite, &route)
            .await?;

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
