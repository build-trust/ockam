use std::sync::Arc;
use std::time::Duration;

use crate::nodes::connection::{Changes, Instantiator};
use crate::{LocalMultiaddrResolver, ReverseLocalConverter};

use ockam::identity::{
    CredentialRetrieverCreator, Identifier, SecureChannelOptions, SecureChannels,
    TrustEveryonePolicy, TrustMultiIdentifiersPolicy,
};
use ockam_core::{async_trait, route, Route};
use ockam_multiaddr::proto::Secure;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

/// Creates secure connection from existing transport
pub(crate) struct SecureChannelInstantiator {
    identifier: Identifier,
    authorized_identities: Option<Vec<Identifier>>,
    timeout: Option<Duration>,
    secure_channels: Arc<SecureChannels>,
    authority: Option<Identifier>,
    credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
}

impl SecureChannelInstantiator {
    pub(crate) fn new(
        identifier: Identifier,
        timeout: Option<Duration>,
        authorized_identities: Option<Vec<Identifier>>,
        authority: Option<Identifier>,
        secure_channels: Arc<SecureChannels>,
        credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
    ) -> Self {
        Self {
            identifier,
            authorized_identities,
            authority,
            timeout,
            secure_channels,
            credential_retriever_creator,
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
    ) -> Result<Changes, ockam_core::Error> {
        let (_before, secure_piece, after) = extracted;
        debug!(%secure_piece, %transport_route, identifier=%self.identifier, "creating secure channel");
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

        let options =
            if let Some(credential_retriever_creator) = self.credential_retriever_creator.clone() {
                options.with_credential_retriever_creator(credential_retriever_creator)?
            } else {
                options
            };

        let options = if let Some(timeout) = self.timeout {
            options.with_timeout(timeout)
        } else {
            options
        };

        let secure_channel = self
            .secure_channels
            .create_secure_channel(
                context,
                &self.identifier,
                route![transport_route, route],
                options,
            )
            .await?;

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
}
