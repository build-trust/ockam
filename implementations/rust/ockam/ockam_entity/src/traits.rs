use crate::credential::{CredentialHolder, CredentialIssuer, CredentialVerifier};
use crate::{
    Contact, ContactsDb, KeyAttributes, ProfileChangeEvent, ProfileEventAttributes,
    ProfileIdentifier, ProfileSync, TrustPolicy,
};
use async_trait::async_trait;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use ockam_vault_core::{PublicKey, Secret};

/// Profile identity.
pub trait ProfileIdentity {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> Result<ProfileIdentifier>;
}

/// Profile verified change history.
pub trait ProfileChanges {
    /// Return change history chain
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>>;
    /// Add a change event.
    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()>;
    /// Verify the whole change event chain
    fn verify(&mut self) -> Result<bool>;
}

/// Profile contact management.
pub trait ProfileContacts {
    /// Return all known to this profile [`Contact`]s
    fn contacts(&self) -> Result<ContactsDb>;
    /// Convert [`Profile`] to [`Contact`]
    fn to_contact(&self) -> Result<Contact>;
    /// Serialize [`Profile`] to [`Contact`] in binary form for storing/transferring over the network
    fn serialize_to_contact(&self) -> Result<Vec<u8>>;
    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>>;
    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact(&mut self, contact: &Contact) -> Result<bool>;
    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool>;
    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool>;
}

/// Profile authentication support.
pub trait ProfileAuth {
    /// Generate an authentication proof based on the given channel_state
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>>;

    /// Verify an authentication proof based on the given channel state, proof and profile.
    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool>;
}

/// Profile secret management.
pub trait ProfileSecrets {
    /// Create new key. Key is uniquely identified by label in [`KeyAttributes`]
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()>;

    /// Rotate existing key. Key is uniquely identified by label in [`KeyAttributes`]
    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()>;

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret>;

    /// Get [`PublicKey`]. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey>;

    /// Get the root [`Secret`]
    fn get_root_secret(&mut self) -> Result<Secret>;
}

/// A trait that represents the two endpoints of a secure channel.
#[async_trait]
pub trait SecureChannelTrait {
    /// Create mutually authenticated secure channel
    async fn create_secure_channel(
        &mut self,
        ctx: &Context,
        route: Route,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<Address>;

    /// Create mutually authenticated secure channel listener
    async fn create_secure_channel_listener(
        &mut self,
        ctx: &Context,
        address: Address,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<()>;
}

/// Supertrait of a Profile
pub trait ProfileTrait:
    ProfileIdentity
    + ProfileChanges
    + ProfileSecrets
    + ProfileContacts
    + ProfileAuth
    + CredentialIssuer
    + CredentialHolder
    + CredentialVerifier
    + Send
    + 'static
{
}

impl<P> ProfileTrait for P where
    P: ProfileIdentity
        + ProfileChanges
        + ProfileSecrets
        + ProfileContacts
        + ProfileAuth
        + CredentialIssuer
        + CredentialHolder
        + CredentialVerifier
        + Send
        + 'static
{
}

pub trait ProfileRetrieve {
    fn profile(&self, profile_identifier: &ProfileIdentifier) -> Option<&ProfileSync>;
    fn profile_mut(&mut self, profile_identifier: &ProfileIdentifier) -> Option<&mut ProfileSync>;
}

pub trait ProfileAdd {
    fn add_profile(&mut self, profile: ProfileSync) -> Result<()>;
}

pub trait ProfileRemove {
    fn remove_profile(&mut self, profile_id: &ProfileIdentifier) -> Result<()>;
}

pub trait ProfileManagement: ProfileAdd + ProfileRemove {}
