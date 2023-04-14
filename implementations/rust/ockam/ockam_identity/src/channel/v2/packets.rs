use crate::channel::v2::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::credential::Credential;
use crate::{IdentityVault, PublicIdentity};
use alloc::sync::Arc;
use ockam_core::vault::Signature;
use ockam_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
//fields ordered by processing priority
pub struct FirstPacket {
    //noise_xx: <- e
    pub key_exchange: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
//fields ordered by processing priority
pub struct SecondPacket {
    //noise_xx: -> e, ee, s, se
    pub(super) key_exchange_with_payload: KeyExchangeWithPayload<IdentityAndCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
//fields ordered by processing priority
pub struct ThirdPacket {
    //noise_xx: -> s, se
    pub(super) key_exchange_with_payload: KeyExchangeWithPayload<IdentityAndCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredential {
    #[serde(flatten)]
    pub(super) identity: EncodedPublicIdentity,
    //signature guarantee that the other end has access to the private key of the identity
    //the hash is the signed content, and it's implicitly known to both parties
    //todo: should we just sign the static key instead of the hash?
    pub(super) signature: Signature,
    pub(super) credential: Option<Credential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct EncodedPublicIdentity {
    encoded: Vec<u8>,
}

impl EncodedPublicIdentity {
    pub(super) fn from(public_identity: &PublicIdentity) -> ockam_core::Result<Self> {
        Ok(Self {
            encoded: public_identity.export()?,
        })
    }

    pub(super) async fn decode(
        &self,
        vault: Arc<dyn IdentityVault>,
    ) -> ockam_core::Result<PublicIdentity> {
        PublicIdentity::import(&self.encoded, vault).await
    }
}
