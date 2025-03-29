use std::time::Duration;

use crate::nodes::connection::{Changes, Instantiator};
use crate::nodes::NodeManager;
use crate::{LocalMultiaddrResolver, ReverseLocalConverter};

use crate::nodes::service::SecureChannelType;
use ockam::identity::Identifier;
use ockam_core::{async_trait, Error, Route, TryClone};
use ockam_multiaddr::proto::Secure;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

/// Creates secure connection from existing transport
pub(crate) struct SecureChannelInstantiator {
    identifier: Identifier,
    authorized_identities: Option<Vec<Identifier>>,
    timeout: Option<Duration>,
}

impl SecureChannelInstantiator {
    pub(crate) fn new(
        identifier: &Identifier,
        timeout: Option<Duration>,
        authorized_identities: Option<Vec<Identifier>>,
    ) -> Self {
        Self {
            identifier: identifier.clone(),
            authorized_identities,
            timeout,
        }
    }
}

#[async_trait]
impl Instantiator for SecureChannelInstantiator {
    fn matches(&self) -> Vec<Match> {
        vec![Secure::CODE.into()]
    }

    async fn instantiate(
        &self,
        ctx: &Context,
        node_manager: &NodeManager,
        transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, Error> {
        let (_before, secure_piece, after) = extracted;
        debug!(%secure_piece, %transport_route, "creating secure channel");
        let route = LocalMultiaddrResolver::resolve(&secure_piece)?;

        let sc_ctx = ctx.try_clone()?;
        let sc = node_manager
            .create_secure_channel_internal(
                &sc_ctx,
                //the transport route is needed to reach the secure channel listener
                //since it can be in another node
                transport_route + route,
                &self.identifier,
                self.authorized_identities.clone(),
                None,
                self.timeout,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        // when creating a secure channel we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr = ReverseLocalConverter::convert_address(sc.encryptor_address())?;
        current_multiaddr.try_extend(after.iter())?;

        Ok(Changes {
            current_multiaddr,
            flow_control_id: Some(sc.flow_control_id().clone()),
            secure_channel_encryptors: vec![sc.encryptor_address().clone()],
            tcp_connection: None,
            udp_bind: None,
        })
    }
}
