use crate::error::ApiError;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    DeleteSecureChannelRequest, DeleteSecureChannelResponse,
};
use crate::nodes::registry::{IdentityRouteKey, SecureChannelInfo};
use crate::nodes::NodeManager;
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::AsyncTryClone;
use ockam_identity::{IdentityIdentifier, TrustMultiIdentifiersPolicy};
use std::sync::Arc;

impl NodeManager {
    pub(super) async fn create_secure_channel_impl<'a>(
        &mut self,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Result<Address> {
        let identity = self.identity()?.async_try_clone().await?;
        let key_route = IdentityRouteKey::new(&identity, &sc_route).await?;

        // If channel was already created, do nothing.
        if let Some(channel) = self.registry.secure_channels.get(&key_route) {
            let addr = channel.addr();
            trace!(%addr, "Using cached secure channel");
            return Ok(addr.clone());
        }

        // Else, create it.
        trace!(%sc_route, "Creating secure channel");
        let sc_addr = match authorized_identifiers {
            Some(ids) => {
                identity
                    .create_secure_channel(
                        sc_route.clone(),
                        TrustMultiIdentifiersPolicy::new(ids),
                        &self.authenticated_storage,
                    )
                    .await
            }
            None => {
                identity
                    .create_secure_channel(
                        sc_route.clone(),
                        TrustEveryonePolicy,
                        &self.authenticated_storage,
                    )
                    .await
            }
        }?;

        trace!(%sc_route, %sc_addr, "Created secure channel");
        // Store the channel using the target route as a key
        let key_route = IdentityRouteKey::new(&identity, &sc_route).await?;
        let v = Arc::new(SecureChannelInfo::new(sc_route, sc_addr.clone()));
        self.registry.secure_channels.insert(key_route, v.clone());

        // Store the channel using its address as a key
        let scr: Route = sc_addr.clone().into();
        let key_addr = IdentityRouteKey::new(&identity, &scr).await?;
        self.registry.secure_channels.insert(key_addr, v);

        // Return secure channel address
        Ok(sc_addr)
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
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let channel = self
            .create_secure_channel_impl(route, authorized_identifiers)
            .await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(&channel));

        Ok(response)
    }

    pub(super) async fn delete_secure_channel<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<DeleteSecureChannelResponse<'a>>> {
        let body: DeleteSecureChannelRequest = dec.decode()?;

        info!(
            "Handling request to delete secure channel: {}",
            body.channel
        );

        let identity = self.identity()?.async_try_clone().await?;

        let sc_address = Address::from(body.channel.as_ref());

        debug!(%sc_address, "Deleting secure channel");

        // Best effort to tear down the channel.
        let _ = identity.stop_secure_channel(&sc_address).await;

        // Remove both the Address and Route entries from the registry.
        let sc_addr = sc_address.clone().into();
        let key_addr = IdentityRouteKey::new(&identity, &sc_addr).await?;
        let res = if let Some(v) = self.registry.secure_channels.remove(&key_addr) {
            trace!(%sc_addr, "Removed secure channel");
            let sc_route = v.route();
            let key_route = IdentityRouteKey::new(&identity, sc_route).await?;
            self.registry.secure_channels.remove(&key_route);
            trace!(%sc_route, "Removed secure channel");
            Some(sc_address)
        } else {
            trace!(%sc_addr, "No secure channels found for the passed address");
            None
        };
        Ok(Response::ok(req.id()).body(DeleteSecureChannelResponse::new(res)))
    }

    pub(super) fn list_secure_channels(
        &mut self,
        req: &Request<'_>,
    ) -> ResponseBuilder<Vec<String>> {
        Response::ok(req.id()).body(
            self.registry
                .secure_channels
                .iter()
                .map(|(_, v)| v.addr().to_string())
                .collect(),
        )
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

        let identity = self.identity()?;

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
    ) -> ResponseBuilder<Vec<String>> {
        Response::ok(req.id()).body(
            self.registry
                .secure_channel_listeners
                .iter()
                .map(|(addr, _)| addr.to_string())
                .collect(),
        )
    }
}
