use tracing::debug;

use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Address, Result};

use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{CredentialRetriever, IdentityError, RemoteCredentialRetriever};

#[async_trait]
impl CredentialRetriever for RemoteCredentialRetriever {
    async fn initialize(&self) -> Result<()> {
        self.initialize_impl().await
    }

    async fn retrieve(&self) -> Result<CredentialAndPurposeKey> {
        debug!(
            "Requested credential for: {} from: {}",
            self.subject, self.issuer_info.issuer
        );

        // Try to get last cached in memory credential
        let last_presented_credential = match self.last_presented_credential.read().unwrap().clone()
        {
            Some(last_presented_credential) => last_presented_credential,
            None => return Err(IdentityError::NoCredential)?,
        };

        let now = now()?;
        // Check if it's still valid
        if last_presented_credential.expires_at > now + self.timing_options.clock_skew_gap {
            // Valid, let's return it
            return Ok(last_presented_credential.credential);
        }

        // TODO: Sometimes worth blocking and waiting for the refresh to happen

        Err(IdentityError::NoCredential)?
    }

    fn subscribe(&self, address: &Address) -> Result<()> {
        let mut subscribers = self.subscribers.write().unwrap();

        if subscribers.contains(address) {
            return Err(IdentityError::AddressAlreadySubscribedForThatCredentialRetriever)?;
        }

        subscribers.push(address.clone());

        Ok(())
    }

    fn unsubscribe(&self, address: &Address) -> Result<()> {
        let mut subscribers = self.subscribers.write().unwrap();

        if let Some(i) = subscribers.iter().position(|x| x == address) {
            subscribers.remove(i);
            Ok(())
        } else {
            Err(IdentityError::AddressIsNotSubscribedForThatCredentialRetriever)?
        }
    }
}
