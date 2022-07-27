use super::map_multiaddr_err;
use crate::error::ApiError;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::NodeMan;
use crate::{Request, Response, ResponseBuilder};
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_identity::{IdentityIdentifier, TrustIdentifierPolicy};
use ockam_multiaddr::MultiAddr;

impl NodeMan {
    pub(super) async fn create_secure_channel_impl<'a>(
        &mut self,
        route: Route,
        known_identifier: Option<IdentityIdentifier>,
    ) -> Result<Address> {
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        let channel = match known_identifier {
            Some(id) => {
                identity
                    .create_secure_channel(
                        route,
                        TrustIdentifierPolicy::new(id),
                        &self.authenticated_storage,
                    )
                    .await
            }
            None => {
                identity
                    .create_secure_channel(route, TrustEveryonePolicy, &self.authenticated_storage)
                    .await
            }
        }?;

        self.registry
            .secure_channels
            .insert(channel.clone(), Default::default());

        Ok(channel)
    }

    pub(super) async fn create_secure_channel<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'a>>> {
        let CreateSecureChannelRequest {
            addr,
            known_identifier,
            ..
        } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        let known_identifier = match known_identifier {
            Some(id) => Some(IdentityIdentifier::try_from(id.0.as_ref())?),
            None => None,
        };

        // TODO: Improve error handling + move logic into CreateSecureChannelRequest
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let channel = self
            .create_secure_channel_impl(route, known_identifier)
            .await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(&channel));

        Ok(response)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        addr: Address,
        known_identifier: Option<IdentityIdentifier>,
    ) -> Result<()> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        match known_identifier {
            Some(id) => {
                identity
                    .create_secure_channel_listener(
                        addr.clone(),
                        TrustIdentifierPolicy::new(id),
                        &self.authenticated_storage,
                    )
                    .await
            }
            None => {
                identity
                    .create_secure_channel_listener(
                        addr.clone(),
                        TrustEveryonePolicy,
                        &self.authenticated_storage,
                    )
                    .await
            }
        }?;

        self.registry
            .secure_channel_listeners
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn create_secure_channel_listener(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let CreateSecureChannelListenerRequest {
            addr,
            known_identifier,
            ..
        } = dec.decode()?;

        let known_identifier = match known_identifier {
            Some(id) => Some(IdentityIdentifier::try_from(id.0.as_ref())?),
            None => None,
        };

        let addr = Address::from(addr.as_ref());
        if !addr.is_local() {
            return Ok(Response::bad_request(req.id()));
        }

        self.create_secure_channel_listener_impl(addr, known_identifier)
            .await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
