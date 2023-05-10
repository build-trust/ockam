use crate::credential::Credential;
use crate::secure_channel::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::{IdentitiesCreation, IdentitiesRepository, IdentitiesVault, Identity};
use alloc::sync::Arc;
use ockam_core::vault::Signature;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct FirstPacket {
    //noise_xx: <- e
    pub(super) key_exchange: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct SecondPacket {
    //noise_xx: -> e, ee, s, es
    pub(super) key_exchange_with_payload: KeyExchangeWithPayload<IdentityAndCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct ThirdPacket {
    //noise_xx: -> s, se
    pub(super) key_exchange_with_payload: KeyExchangeWithPayload<IdentityAndCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredential {
    pub(super) identity: EncodedPublicIdentity,
    //signature guarantee that the other end has access to the private key of the identity
    //the signature refers to the static key of the noise ('x') and is made with the static
    //key of the identity
    pub(super) signature: Signature,
    pub(super) credentials: Vec<Credential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct EncodedPublicIdentity {
    encoded: Vec<u8>,
}

impl EncodedPublicIdentity {
    pub(super) fn from(public_identity: &Identity) -> ockam_core::Result<Self> {
        Ok(Self {
            encoded: public_identity.export()?,
        })
    }

    pub(super) async fn decode(
        &self,
        repository: Arc<dyn IdentitiesRepository>,
        vault: Arc<dyn IdentitiesVault>,
    ) -> ockam_core::Result<Identity> {
        IdentitiesCreation::new(repository, vault)
            .decode_identity(&self.encoded)
            .await
    }
}
