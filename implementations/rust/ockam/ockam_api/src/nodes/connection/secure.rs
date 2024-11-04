use std::sync::Arc;
use std::time::Duration;

use crate::nodes::connection::{Changes, Connection, Instantiator};
use crate::{LocalMultiaddrResolver, ReverseLocalConverter};

use crate::nodes::registry::SecureChannelRegistry;
use ockam::identity::{CredentialRetrieverCreator, Identifier};
use ockam::identity::{
    SecureChannelOptions, SecureChannels, TrustEveryonePolicy, TrustMultiIdentifiersPolicy,
};
use ockam_core::{async_trait, Error};
use ockam_core::{Route, TryClone};
use ockam_multiaddr::proto::Secure;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

/// Creates secure connection from existing transport
pub struct SecureChannelInstantiator {
    identifier: Identifier,
    authorized_identities: Option<Vec<Identifier>>,
    timeout: Option<Duration>,
    secure_channels: Arc<SecureChannels>,
    authority: Option<Identifier>,
    credential_retriever: Option<Arc<dyn CredentialRetrieverCreator>>,
    secure_channel_registry: Option<Arc<SecureChannelRegistry>>,
}

impl SecureChannelInstantiator {
    pub fn new(
        identifier: &Identifier,
        timeout: Option<Duration>,
        authorized_identities: Option<Vec<Identifier>>,
        authority: Option<Identifier>,
        secure_channels: Arc<SecureChannels>,
        credential_retriever: Option<Arc<dyn CredentialRetrieverCreator>>,
        secure_channel_registry: Option<Arc<SecureChannelRegistry>>,
    ) -> Self {
        Self {
            identifier: identifier.clone(),
            credential_retriever,
            authorized_identities,
            authority,
            timeout,
            secure_channels,
            secure_channel_registry,
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
        context: &Context,
        transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, Error> {
        let (_before, secure_piece, after) = extracted;
        debug!(%secure_piece, %transport_route, "creating secure channel");
        let route = LocalMultiaddrResolver::resolve(&secure_piece)?;

        let options = SecureChannelOptions::new();

        let options = match self.authorized_identities.clone() {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let options = if let Some(authority) = self.authority.clone() {
            options.with_authority(authority)
        } else {
            options
        };

        let options = if let Some(timeout) = self.timeout {
            options.with_timeout(timeout)
        } else {
            options
        };

        let options = match &self.credential_retriever {
            None => options,
            Some(retriever) => options.with_credential_retriever_creator(retriever.clone())?,
        };

        let secure_channel_context = context.try_clone()?;
        let route = transport_route + route;
        let secure_channel = self
            .secure_channels
            .create_secure_channel(
                &secure_channel_context,
                &self.identifier,
                route.clone(),
                options,
            )
            .await?;

        if let Some(registry) = &self.secure_channel_registry {
            registry.insert(
                route,
                secure_channel.clone(),
                self.authorized_identities.clone(),
            );
        }

        // when creating a secure channel we want the route to pass through that
        // ignoring previous steps, since they will be implicit
        let mut current_multiaddr =
            ReverseLocalConverter::convert_address(secure_channel.encryptor_address())?;
        current_multiaddr.try_extend(after.iter())?;

        Ok(Changes {
            current_multiaddr,
            flow_control_id: Some(secure_channel.flow_control_id().clone()),
            secure_channel_encryptors: vec![secure_channel.encryptor_address().clone()],
            tcp_connection: None,
            udp_bind: None,
        })
    }

    async fn close(&self, context: &Context, connection: &Connection) {
        for encryptor_address in &connection.secure_channel_encryptors {
            if let Some(registry) = &self.secure_channel_registry {
                registry.remove_by_addr(encryptor_address);
            }

            if let Err(error) = self
                .secure_channels
                .stop_secure_channel(context, encryptor_address)
            {
                warn!(%error, "Failed to stop secure channel");
            }
        }
    }
}
