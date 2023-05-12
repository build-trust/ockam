use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::{Addresses, Role};
use crate::{to_xx_initialized, Identity, SecureChannelRegistryEntry, SecureChannels};
use ockam_core::{
    AllowAll, AllowOnwardAddress, CompletedKeyExchange, LocalOnwardOnly, LocalSourceOnly, Mailbox,
    Mailboxes, Route,
};
use ockam_node::{Context, WorkerBuilder};
use std::sync::Arc;
use tracing::info;

pub(crate) struct Finalizer {
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) identity: Identity,
    pub(crate) their_identity: Identity,
    pub(crate) keys: CompletedKeyExchange,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
}

impl Finalizer {
    /// Performs the last steps of the secure channel establishment
    pub(crate) async fn finalize(
        self,
        context: &mut Context,
        role: Role,
    ) -> ockam_core::Result<DecryptorWorker> {
        //decryptor worker
        let decryptor = DecryptorWorker::new(
            role.str(),
            self.addresses.clone(),
            Decryptor::new(
                self.keys.decrypt_key().clone(),
                to_xx_initialized(self.secure_channels.identities.vault()),
            ),
            self.their_identity.identifier(),
        );

        //encryptor worker
        {
            let encryptor = EncryptorWorker::new(
                role.str(),
                self.addresses.clone(),
                self.remote_route.clone(),
                Encryptor::new(
                    self.keys.encrypt_key().clone(),
                    0,
                    to_xx_initialized(self.secure_channels.identities.vault()),
                ),
            );

            let next_hop = self.remote_route.next()?.clone();
            let main_mailbox = Mailbox::new(
                self.addresses.encryptor.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowOnwardAddress(next_hop)),
            );
            let api_mailbox = Mailbox::new(
                self.addresses.encryptor_api.clone(),
                Arc::new(LocalSourceOnly),
                Arc::new(LocalOnwardOnly),
            );

            WorkerBuilder::with_mailboxes(
                Mailboxes::new(main_mailbox, vec![api_mailbox]),
                encryptor,
            )
            .start(context)
            .await?;
        }

        info!(
            "Initialized SecureChannel {} at local: {}, remote: {}",
            role.str(),
            &self.addresses.encryptor,
            &self.addresses.decryptor_remote
        );

        let info = SecureChannelRegistryEntry::new(
            self.addresses.encryptor.clone(),
            self.addresses.encryptor_api.clone(),
            self.addresses.decryptor_remote.clone(),
            self.addresses.decryptor_api.clone(),
            role.is_initiator(),
            self.identity.identifier(),
            self.their_identity.identifier(),
        );

        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }
}
