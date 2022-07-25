use super::map_multiaddr_err;
use crate::error::ApiError;
use crate::nodes::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::NodeMan;
use crate::{Request, Response, ResponseBuilder};
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_multiaddr::MultiAddr;

impl NodeMan {
    pub(super) async fn create_secure_channel_impl<'a>(&mut self, route: Route) -> Result<Address> {
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        let channel = identity
            .create_secure_channel(route, TrustEveryonePolicy, &self.authenticated_storage)
            .await?;

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
        let CreateSecureChannelRequest { addr, .. } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        // TODO: Improve error handling + move logic into CreateSecureChannelRequest
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let channel = self.create_secure_channel_impl(route).await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(channel));

        Ok(response)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        addr: Address,
    ) -> Result<()> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        identity
            .create_secure_channel_listener(
                addr.clone(),
                TrustEveryonePolicy,
                &self.authenticated_storage,
            )
            .await?;

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
        let CreateSecureChannelListenerRequest { addr, .. } = dec.decode()?;

        let addr = Address::from(addr.as_ref());
        if !addr.is_local() {
            return Ok(Response::bad_request(req.id()));
        }

        self.create_secure_channel_listener_impl(addr).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
