use super::map_multiaddr_err;
use crate::error::ApiError;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    SecureChannelListenerAddrList,
};
use crate::nodes::NodeManager;
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_identity::{IdentityIdentifier, TrustMultiIdentifiersPolicy};
use ockam_multiaddr::MultiAddr;

impl NodeManager {
    pub(super) async fn create_secure_channel_impl<'a>(
        &mut self,
        route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Result<Address> {
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        let channel = match authorized_identifiers {
            Some(ids) => {
                identity
                    .create_secure_channel(
                        route,
                        TrustMultiIdentifiersPolicy::new(ids),
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
            authorized_identifiers,
            ..
        } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        let authorized_identifiers = match authorized_identifiers {
            Some(ids) => {
                let ids = ids
                    .into_iter()
                    .map(|x| IdentityIdentifier::try_from(x.0.as_ref()))
                    .collect::<Result<Vec<IdentityIdentifier>>>()?;

                Some(ids)
            }
            None => None,
        };

        // TODO: Improve error handling + move logic into CreateSecureChannelRequest
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let channel = self
            .create_secure_channel_impl(route, authorized_identifiers)
            .await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(&channel));

        Ok(response)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        addr: Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Result<()> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        match authorized_identifiers {
            Some(ids) => {
                identity
                    .create_secure_channel_listener(
                        addr.clone(),
                        TrustMultiIdentifiersPolicy::new(ids),
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
            authorized_identifiers,
            ..
        } = dec.decode()?;

        let authorized_identifiers = match authorized_identifiers {
            Some(ids) => {
                let ids = ids
                    .into_iter()
                    .map(|x| IdentityIdentifier::try_from(x.0.as_ref()))
                    .collect::<Result<Vec<IdentityIdentifier>>>()?;

                Some(ids)
            }
            None => None,
        };

        let addr = Address::from(addr.as_ref());
        if !addr.is_local() {
            return Ok(Response::bad_request(req.id()));
        }

        self.create_secure_channel_listener_impl(addr, authorized_identifiers)
            .await?;

        let response = Response::ok(req.id());

        Ok(response)
    }

    pub(super) fn list_secure_channel_listener(
        &mut self,
        req: &Request<'_>,
    ) -> ResponseBuilder<SecureChannelListenerAddrList> {
        Response::ok(req.id()).body(SecureChannelListenerAddrList::new(
            self.registry
                .secure_channel_listeners
                .iter()
                .map(|(addr, _)| addr.to_string())
                .collect(),
        ))
    }
}
