use crate::{
    Contact, ContactsDb, Entity, KeyAttributes, Profile, ProfileAuth, ProfileChangeEvent,
    ProfileChanges, ProfileContacts, ProfileEventAttributes, ProfileIdentifier, ProfileIdentity,
    ProfileSecrets, ProfileSync, RemoteEntity, RemoteForwarder, RemoteForwarderInfo, Route,
    SecureChannelTrait,
};
use ockam_core::{Address, AddressSet, Message, Result, Worker};
use ockam_node::{Cancel, Context};
use ockam_vault_core::{PublicKey, Secret};
use ockam_vault_sync_core::Vault;

use ockam_core::hashbrown::HashMap;

pub struct LocalEntity {
    pub ctx: Context,
    vault: Address,
    entity: Entity<ProfileSync>,
    secure_channels: HashMap<Address, Route>,
}

impl LocalEntity {
    pub async fn create<A: Into<Address>>(node_ctx: &Context, address: A) -> Result<LocalEntity> {
        let ctx = node_ctx.new_context(address).await?;

        let vault = Vault::create(&ctx)?;
        let default_profile = Profile::create(&ctx, &vault).await?;
        let entity = Entity::new(default_profile);

        Ok(LocalEntity {
            ctx,
            vault,
            entity,
            secure_channels: Default::default(),
        })
    }

    /// Create with a given worker
    pub async fn create_with_worker<A, M, W>(
        ctx: &Context,
        address: A,
        worker: W,
    ) -> Result<LocalEntity>
    where
        A: Into<AddressSet>,
        M: Message + Send + 'static,
        W: Worker<Context = Context, Message = M>,
    {
        #[allow(clippy::needless_lifetimes)]
        let local = LocalEntity::create(ctx, Address::random(0)).await?;

        ctx.start_worker(address, worker).await?;

        Ok(local)
    }

    pub async fn secure_channel_listen_on_address<A: Into<Address>>(
        &mut self,
        address: A,
    ) -> Result<()> {
        self.entity
            .create_secure_channel_listener(&self.ctx, address.into(), &self.vault)
            .await
    }

    pub fn secure_channel_address(&self) -> String {
        "secure_channel_listener".to_string()
    }

    pub async fn create_secure_channel_listener(
        &mut self,
        secure_channel_address: &str,
    ) -> Result<()> {
        self.secure_channel_listen_on_address(secure_channel_address)
            .await
    }

    pub async fn create_secure_channel<R: Into<Route>>(&mut self, route: R) -> Result<Address> {
        let route = route.into();
        let channel = self
            .entity
            .create_secure_channel(&self.ctx, route.clone(), &self.vault)
            .await?;

        self.secure_channels.insert(channel.clone(), route);

        Ok(channel)
    }

    pub fn list_secure_channels(&self) -> Result<Vec<(&Address, &Route)>> {
        Ok(self.secure_channels.iter().collect())
    }

    pub async fn forward<A: Into<Address>>(
        &mut self,
        remote_entity: RemoteEntity,
        service_address: A,
    ) -> Result<RemoteForwarderInfo> {
        let address = remote_entity.route.next().unwrap().to_string();
        let address = address.strip_prefix("0#").unwrap(); // TODO how can we clean this up?
        RemoteForwarder::create(&self.ctx, address, service_address.into()).await
    }

    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.ctx.send(route, msg).await
    }

    pub async fn receive<'ctx, M: Message>(&'ctx mut self) -> Result<Cancel<'ctx, M>> {
        self.ctx.receive().await
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.ctx.stop().await
    }
}

impl ProfileIdentity for LocalEntity {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        self.entity.identifier()
    }
}

impl ProfileChanges for LocalEntity {
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

impl ProfileSecrets for LocalEntity {
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

impl ProfileContacts for LocalEntity {
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

impl ProfileAuth for LocalEntity {
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
