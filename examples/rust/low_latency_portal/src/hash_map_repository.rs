use async_trait::async_trait;
use ockam::compat::asynchronous::RwLock as AsyncRwLock;
use ockam::compat::collections::HashMap;
use ockam::compat::sync::RwLock as SyncRwLock;
use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey, PurposeKeyAttestation};
use ockam::identity::storage::PurposeKeysRepository;
use ockam::identity::{
    AttributesEntry, ChangeHistoryRepository, CredentialRepository, Identifier, Identity, IdentityAttributesRepository,
    IdentityError, IdentityHistoryComparison, PersistedSecureChannel, Purpose, SecureChannelRepository,
    TimestampInSeconds, Vault,
};
use ockam::vault::storage::SecretsRepository;
use ockam::vault::{
    AeadSecret, AeadSecretKeyHandle, SigningSecret, SigningSecretKeyHandle, X25519SecretKey, X25519SecretKeyHandle,
};
use ockam::{Address, Result};

#[derive(Default)]
pub struct HashMapRepository {
    signing_secrets: SyncRwLock<HashMap<SigningSecretKeyHandle, SigningSecret>>,
    x25519_secrets: SyncRwLock<HashMap<X25519SecretKeyHandle, X25519SecretKey>>,
    aead_secrets: SyncRwLock<HashMap<AeadSecretKeyHandle, AeadSecret>>,

    secure_channels: SyncRwLock<HashMap<Address, PersistedSecureChannel>>,

    identities: AsyncRwLock<HashMap<Identifier, ChangeHistory>>,

    #[allow(clippy::type_complexity)]
    attributes: SyncRwLock<HashMap<Identifier, Vec<(Option<Identifier>, AttributesEntry)>>>,

    purpose_keys: SyncRwLock<HashMap<Identifier, Vec<(Purpose, PurposeKeyAttestation)>>>,

    #[allow(clippy::type_complexity)]
    credentials:
        SyncRwLock<HashMap<Identifier, Vec<(Identifier, String, TimestampInSeconds, CredentialAndPurposeKey)>>>,
}

#[async_trait]
impl SecretsRepository for HashMapRepository {
    async fn store_signing_secret(&self, handle: &SigningSecretKeyHandle, secret: SigningSecret) -> Result<()> {
        self.signing_secrets.write().unwrap().insert(handle.clone(), secret);

        Ok(())
    }

    async fn delete_signing_secret(&self, handle: &SigningSecretKeyHandle) -> Result<bool> {
        Ok(self.signing_secrets.write().unwrap().remove(handle).is_some())
    }

    async fn get_signing_secret(&self, handle: &SigningSecretKeyHandle) -> Result<Option<SigningSecret>> {
        Ok(self.signing_secrets.read().unwrap().get(handle).cloned())
    }

    async fn get_signing_secret_handles(&self) -> Result<Vec<SigningSecretKeyHandle>> {
        Ok(self.signing_secrets.read().unwrap().keys().cloned().collect())
    }

    async fn store_x25519_secret(&self, handle: &X25519SecretKeyHandle, secret: X25519SecretKey) -> Result<()> {
        self.x25519_secrets.write().unwrap().insert(handle.clone(), secret);

        Ok(())
    }

    async fn delete_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<bool> {
        Ok(self.x25519_secrets.write().unwrap().remove(handle).is_some())
    }

    async fn get_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<Option<X25519SecretKey>> {
        Ok(self.x25519_secrets.read().unwrap().get(handle).cloned())
    }

    async fn get_x25519_secret_handles(&self) -> Result<Vec<X25519SecretKeyHandle>> {
        Ok(self.x25519_secrets.read().unwrap().keys().cloned().collect())
    }

    async fn store_aead_secret(&self, handle: &AeadSecretKeyHandle, secret: AeadSecret) -> Result<()> {
        self.aead_secrets.write().unwrap().insert(handle.clone(), secret);

        Ok(())
    }

    async fn delete_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<bool> {
        Ok(self.aead_secrets.write().unwrap().remove(handle).is_some())
    }

    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<Option<AeadSecret>> {
        Ok(self.aead_secrets.read().unwrap().get(handle).cloned())
    }

    async fn delete_all(&self) -> Result<()> {
        self.signing_secrets.write().unwrap().clear();
        self.x25519_secrets.write().unwrap().clear();
        self.aead_secrets.write().unwrap().clear();

        Ok(())
    }
}

#[async_trait]
impl SecureChannelRepository for HashMapRepository {
    async fn get(&self, decryptor_remote_address: &Address) -> Result<Option<PersistedSecureChannel>> {
        Ok(self
            .secure_channels
            .read()
            .unwrap()
            .get(decryptor_remote_address)
            .cloned())
    }

    async fn put(&self, secure_channel: PersistedSecureChannel) -> Result<()> {
        self.secure_channels
            .write()
            .unwrap()
            .insert(secure_channel.decryptor_remote().clone(), secure_channel);

        Ok(())
    }

    async fn delete(&self, decryptor_remote_address: &Address) -> Result<()> {
        self.secure_channels.write().unwrap().remove(decryptor_remote_address);

        Ok(())
    }
}

#[async_trait]
impl ChangeHistoryRepository for HashMapRepository {
    async fn update_identity(&self, identity: &Identity, ignore_older: bool) -> Result<()> {
        let mut identities = self.identities.write().await;

        let do_insert = match identities.get(identity.identifier()) {
            Some(existing_identity) => {
                let known_identity = Identity::import_from_change_history(
                    Some(identity.identifier()),
                    existing_identity.clone(),
                    Vault::create_verifying_vault(),
                )
                .await?;

                match identity.compare(&known_identity) {
                    IdentityHistoryComparison::Conflict => {
                        return Err(IdentityError::ConsistencyError)?;
                    }
                    IdentityHistoryComparison::Older => {
                        if ignore_older {
                            false
                        } else {
                            return Err(IdentityError::ConsistencyError)?;
                        }
                    }

                    IdentityHistoryComparison::Newer => true,
                    IdentityHistoryComparison::Equal => false,
                }
            }
            None => true,
        };

        if do_insert {
            identities.insert(identity.identifier().clone(), identity.change_history().clone());
        }

        Ok(())
    }

    async fn store_change_history(&self, identifier: &Identifier, change_history: ChangeHistory) -> Result<()> {
        self.identities.write().await.insert(identifier.clone(), change_history);

        Ok(())
    }

    async fn delete_change_history(&self, identifier: &Identifier) -> Result<()> {
        self.identities.write().await.remove(identifier);

        Ok(())
    }

    async fn get_change_history(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        Ok(self.identities.read().await.get(identifier).cloned())
    }

    async fn get_change_histories(&self) -> Result<Vec<ChangeHistory>> {
        Ok(self.identities.read().await.values().cloned().collect())
    }
}

#[async_trait]
impl IdentityAttributesRepository for HashMapRepository {
    async fn get_attributes(&self, subject: &Identifier, attested_by: &Identifier) -> Result<Option<AttributesEntry>> {
        Ok(self.attributes.read().unwrap().get(subject).and_then(|attrs| {
            attrs.iter().find_map(|(e_identifier, e_entry)| {
                if e_identifier.as_ref() == Some(attested_by) {
                    Some(e_entry.clone())
                } else {
                    None
                }
            })
        }))
    }

    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        self.attributes
            .write()
            .unwrap()
            .entry(subject.clone())
            .or_default()
            .push((entry.attested_by(), entry));

        Ok(())
    }

    async fn delete_expired_attributes(&self, now: TimestampInSeconds) -> Result<()> {
        for value in self.attributes.write().unwrap().values_mut() {
            _ = value.retain(|entry| entry.1.expires_at() > Some(now));
        }

        Ok(())
    }
}

#[async_trait]
impl PurposeKeysRepository for HashMapRepository {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()> {
        self.purpose_keys
            .write()
            .unwrap()
            .entry(subject.clone())
            .or_default()
            .push((purpose, purpose_key_attestation.clone()));

        Ok(())
    }

    async fn delete_purpose_key(&self, subject: &Identifier, _purpose: Purpose) -> Result<()> {
        self.purpose_keys.write().unwrap().remove(subject);

        Ok(())
    }

    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>> {
        Ok(self.purpose_keys.read().unwrap().get(identifier).and_then(|e| {
            e.iter().find_map(|(e_purpose, attestation)| {
                if e_purpose == &purpose {
                    Some(attestation.clone())
                } else {
                    None
                }
            })
        }))
    }

    async fn delete_all(&self) -> Result<()> {
        self.purpose_keys.write().unwrap().clear();

        Ok(())
    }
}

#[async_trait]
impl CredentialRepository for HashMapRepository {
    async fn get(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        if let Some(e) = self.credentials.read().unwrap().get(subject) {
            return Ok(e.iter().find_map(|(e_issuer, e_scope, _expires, cred)| {
                if e_issuer == issuer && e_scope == scope {
                    Some(cred.clone())
                } else {
                    None
                }
            }));
        }

        Ok(None)
    }

    async fn put(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
        expires_at: TimestampInSeconds,
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        self.credentials
            .write()
            .unwrap()
            .entry(subject.clone())
            .or_default()
            .push((issuer.clone(), scope.to_string(), expires_at, credential.clone()));

        Ok(())
    }

    async fn delete(&self, subject: &Identifier, issuer: &Identifier, scope: &str) -> Result<()> {
        if let Some(e) = self.credentials.write().unwrap().get_mut(subject) {
            e.retain(|(e_issuer, e_scope, _e_expires, _e_cred)| e_issuer != issuer || e_scope != scope);
        }

        Ok(())
    }
}
