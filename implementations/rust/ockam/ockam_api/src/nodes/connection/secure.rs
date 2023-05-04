use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionInstanceBuilder, Instantiator};
use crate::nodes::models::secure_channel::CredentialExchangeMode;
use crate::nodes::NodeManager;
use crate::{local_multiaddr_to_route, try_address_to_multiaddr};

use ockam::compat::tokio::sync::RwLock;
use ockam_core::{async_trait, route, Error};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::Secure;
use ockam_multiaddr::{Match, Protocol};
use ockam_node::Context;

use ockam_core::compat::sync::Arc;
use std::time::Duration;

/// Creates secure connection from existing transport
pub(crate) struct SecureChannelInstantiator {
    node_manager: Arc<RwLock<NodeManager>>,
    timeout: Option<Duration>,
    context: Arc<Context>,
    authorized_identities: Option<Vec<IdentityIdentifier>>,
}

impl SecureChannelInstantiator {
    pub(crate) fn new(
        context: Arc<Context>,
        node_manager: Arc<RwLock<NodeManager>>,
        timeout: Option<Duration>,
        authorized_identities: Option<Vec<IdentityIdentifier>>,
    ) -> Self {
        Self {
            authorized_identities,
            context,
            node_manager,
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
        builder: &ConnectionInstanceBuilder,
        match_start: usize,
    ) -> Result<Changes, Error> {
        let (_before, secure_piece, after) =
            ConnectionInstanceBuilder::extract(&builder.current_multiaddr, match_start, 1);

        let transport_route = builder.transport_route.clone();
        debug!(%secure_piece, %transport_route, "creating secure channel");
        let route = local_multiaddr_to_route(&secure_piece)
            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;

        let mut node_manager = self.node_manager.write().await;
        let sc = node_manager
            .create_secure_channel_impl(
                //the transport route is needed to reach the secure channel listener
                //since it can be in another node
                route![transport_route, route],
                self.authorized_identities.clone(),
                CredentialExchangeMode::Mutual,
                self.timeout,
                None,
                &self.context,
                None,
            )
            .await?;

        // when creating a secure channel we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr = try_address_to_multiaddr(sc.encryptor_address()).unwrap();
        current_multiaddr.try_extend(after.iter())?;

        Ok(Changes {
            current_multiaddr,
            flow_control_id: Some(sc.flow_control_id().clone()),
            secure_channel_encryptors: vec![sc.encryptor_address().clone()],
            tcp_worker: None,
        })
    }
}
