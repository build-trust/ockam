use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::{Addresses, Role};
use crate::{
    to_xx_initialized, Credential, Credentials, Identity, IdentityError, IdentityIdentifier,
    SecureChannelRegistryEntry, SecureChannelTrustInfo, SecureChannels, TrustContext, TrustPolicy,
};
use ockam_core::vault::Signature;
use ockam_core::{
    AllowAll, AllowOnwardAddress, CompletedKeyExchange, LocalOnwardOnly, LocalSourceOnly, Mailbox,
    Mailboxes, Route,
};
use ockam_node::{Context, WorkerBuilder};
use std::sync::Arc;
use tracing::info;

pub(crate) struct Finalizer {
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) signature: Signature,
    pub(crate) identifier: IdentityIdentifier,
    pub(crate) their_identity: Identity,
    pub(crate) keys: CompletedKeyExchange,
    pub(crate) credentials: Vec<Credential>,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl Finalizer {
    /// Performs the last steps of the secure channel establishment
    pub(crate) async fn finalize(
        self,
        context: &mut Context,
        role: Role,
    ) -> ockam_core::Result<DecryptorWorker> {
        //verify the signature of the static key used during noise exchanges
        //actually matches the signature of the identity
        let signature_verified = self
            .secure_channels
            .identities
            .identities_keys()
            .verify_signature(
                &self.their_identity,
                &self.signature,
                self.keys.public_static_key().data(),
                None,
            )
            .await?;

        if !signature_verified {
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        }

        // Check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(self.their_identity.identifier.clone());
        let trusted = self.trust_policy.check(&trust_info).await?;
        if !trusted {
            // TODO: Shutdown? Communicate error?
            return Err(IdentityError::SecureChannelTrustCheckFailed.into());
        }
        info!(
            "Initiator checked trust policy for SecureChannel from: {}",
            self.their_identity.identifier
        );

        if let Some(trust_context) = self.trust_context {
            for credential in self.credentials {
                let result = self
                    .secure_channels
                    .identities()
                    .receive_presented_credential(
                        &self.their_identity.identifier,
                        &[trust_context.authority()?.identity().await?],
                        credential,
                    )
                    .await;

                if let Some(_err) = result.err() {
                    //TODO: consider the possibility of keep going when a credential validation fails
                    return Err(IdentityError::SecureChannelVerificationFailed.into());
                }
            }
        } else if !self.credentials.is_empty() {
            //we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        }

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
            self.identifier,
            self.their_identity.identifier(),
        );

        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }
}
