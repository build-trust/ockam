use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::{Addresses, Role};
use crate::{
    to_xx_initialized, Credential, Credentials, Identity, IdentityError, IdentityIdentifier,
    SecureChannelRegistryEntry, SecureChannelTrustInfo, SecureChannels, TrustContext, TrustPolicy,
};
use alloc::vec::Vec;
use ockam_core::compat::sync::Arc;
use ockam_core::{AllowAll, AllowOnwardAddress, CompletedKeyExchange, Mailbox, Mailboxes, Route};
use ockam_node::{Context, WorkerBuilder};
use ockam_vault::Signature;
use tracing::info;

pub(crate) struct ExchangeCompleter {
    pub(crate) role: Role,
    pub(crate) identity_identifier: IdentityIdentifier,
    pub(crate) keys: CompletedKeyExchange,
    pub(crate) their_signature: Signature,
    pub(crate) their_identity: Identity,
    pub(crate) their_credentials: Vec<Credential>,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl ExchangeCompleter {
    /// Performs the last steps of the secure channel establishment
    pub(crate) async fn complete(
        self,
        context: &mut Context,
        secure_channels: Arc<SecureChannels>,
    ) -> ockam_core::Result<DecryptorWorker> {
        //verify the signature of the static key used during noise exchanges
        //actually matches the signature of the identity
        let signature_verified = secure_channels
            .identities
            .identities_keys()
            .verify_signature(
                &self.their_identity,
                &self.their_signature,
                self.keys.public_static_key(),
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
            if self.their_credentials.len() >= 2 {
                //FIXME: remove as soon as we start supporting multiple credentials
                return Err(IdentityError::CredentialVerificationFailed.into());
            }

            for credential in self.their_credentials {
                let result = secure_channels
                    .identities()
                    .receive_presented_credential(
                        &self.their_identity.identifier,
                        &[trust_context.authority()?.identity().await?],
                        credential,
                    )
                    .await;

                if let Some(_err) = result.err() {
                    //TODO: consider the possibility of keep going when a credential validation fails
                    return Err(IdentityError::CredentialVerificationFailed.into());
                }
            }
        } else if !self.their_credentials.is_empty() {
            //we cannot validate credentials without a trust context
            return Err(IdentityError::UnknownAuthority.into());
        }

        //store identity for future validation
        secure_channels
            .identities()
            .repository()
            .update_identity(&self.their_identity)
            .await?;

        //decryptor worker
        let decryptor = DecryptorWorker::new(
            self.role.str(),
            self.addresses.clone(),
            Decryptor::new(
                self.keys.decrypt_key().clone(),
                to_xx_initialized(secure_channels.identities.vault()),
            ),
            self.their_identity.identifier(),
        );

        //encryptor worker
        {
            let encryptor = EncryptorWorker::new(
                self.role.str(),
                self.addresses.clone(),
                self.remote_route.clone(),
                Encryptor::new(
                    self.keys.encrypt_key().clone(),
                    0,
                    to_xx_initialized(secure_channels.identities.vault()),
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
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            );

            WorkerBuilder::new(encryptor)
                .with_mailboxes(Mailboxes::new(main_mailbox, vec![api_mailbox]))
                .start(context)
                .await?;
        }

        info!(
            "Initialized SecureChannel {} at local: {}, remote: {}",
            self.role.str(),
            &self.addresses.encryptor,
            &self.addresses.decryptor_remote
        );

        let info = SecureChannelRegistryEntry::new(
            self.addresses.encryptor.clone(),
            self.addresses.encryptor_api.clone(),
            self.addresses.decryptor_remote.clone(),
            self.addresses.decryptor_api.clone(),
            self.role.is_initiator(),
            self.identity_identifier,
            self.their_identity.identifier(),
        );

        secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }
}
