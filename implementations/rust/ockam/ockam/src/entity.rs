use crate::{
    Contact, ContactsDb, KeyAttributes, Profile, ProfileAuth, ProfileChangeEvent, ProfileChanges,
    ProfileContacts, ProfileEventAttributes, ProfileIdentifier, ProfileIdentity, ProfileSecrets,
    ProfileSet, ProfileSync, Route, SecureChannelTrait, TrustPolicy,
};
use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_vault_core::{PublicKey, Secret};
use ockam_vault_sync_core::Vault;

use ockam_core::hashbrown::HashMap;

pub struct Entity {
    ctx: Context,
    vault: Address,
    entity: ProfileSet<ProfileSync>,
    secure_channels: HashMap<Address, Route>,
}

impl Entity {
    pub async fn create(node_ctx: &Context) -> Result<Entity> {
        let ctx = node_ctx.new_context(Address::random(0)).await?;
        let vault = Vault::create(&ctx)?;
        let default_profile = Profile::create(&ctx, &vault).await?;
        let entity = ProfileSet::new(default_profile);

        Ok(Entity {
            ctx,
            vault,
            entity,
            secure_channels: Default::default(),
        })
    }

    pub async fn secure_channel_listen_on_address<A: Into<Address>, T: TrustPolicy>(
        &mut self,
        address: A,
        trust_policy: T,
    ) -> Result<()> {
        self.entity
            .create_secure_channel_listener(&self.ctx, address.into(), trust_policy, &self.vault)
            .await
    }

    pub async fn create_secure_channel_listener<T: TrustPolicy>(
        &mut self,
        secure_channel_address: &str,
        trust_policy: T,
    ) -> Result<()> {
        self.secure_channel_listen_on_address(secure_channel_address, trust_policy)
            .await
    }

    pub async fn create_secure_channel<R: Into<Route>, T: TrustPolicy>(
        &mut self,
        route: R,
        trust_policy: T,
    ) -> Result<Address> {
        let route = route.into();
        let channel = self
            .entity
            .create_secure_channel(&self.ctx, route.clone(), trust_policy, &self.vault)
            .await?;

        self.secure_channels.insert(channel.clone(), route);

        Ok(channel)
    }

    pub fn list_secure_channels(&self) -> Result<Vec<(&Address, &Route)>> {
        Ok(self.secure_channels.iter().collect())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.ctx.stop().await
    }
}

impl ProfileIdentity for Entity {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        self.entity.identifier()
    }
}

impl ProfileChanges for Entity {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        self.entity.change_events()
    }

    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        self.entity.update_no_verification(change_event)
    }

    fn verify(&mut self) -> Result<bool> {
        self.entity.verify()
    }
}

impl ProfileSecrets for Entity {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        self.entity.create_key(key_attributes, attributes)
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        self.entity.rotate_key(key_attributes, attributes)
    }

    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        self.entity.get_secret_key(key_attributes)
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        self.entity.get_public_key(key_attributes)
    }

    fn get_root_secret(&mut self) -> Result<Secret> {
        self.entity.get_root_secret()
    }
}

impl ProfileContacts for Entity {
    fn contacts(&self) -> Result<ContactsDb> {
        self.entity.contacts()
    }

    fn to_contact(&self) -> Result<Contact> {
        self.entity.to_contact()
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        self.entity.serialize_to_contact()
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        self.entity.get_contact(id)
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        self.entity.verify_contact(contact)
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        self.entity.verify_and_add_contact(contact)
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        self.entity
            .verify_and_update_contact(profile_id, change_events)
    }
}

impl ProfileAuth for Entity {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        self.entity.generate_authentication_proof(channel_state)
    }

    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        self.entity
            .verify_authentication_proof(channel_state, responder_contact_id, proof)
    }
}
