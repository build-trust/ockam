use crate::{EntityError, ProfileVault};
use ockam_vault_core::{PublicKey, Secret};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthenticationProof {
    #[serde(with = "BigArray")]
    signature: [u8; 64],
}

impl AuthenticationProof {
    pub(crate) fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

impl AuthenticationProof {
    pub(crate) fn new(signature: [u8; 64]) -> Self {
        AuthenticationProof { signature }
    }
}

pub(crate) struct Authentication {}

impl Authentication {
    pub(crate) fn generate_proof<V: ProfileVault>(
        channel_state: &[u8],
        secret: &Secret,
        vault: &mut V,
    ) -> ockam_core::Result<Vec<u8>> {
        let signature = vault.sign(secret, channel_state)?;

        let proof = AuthenticationProof::new(signature);

        serde_bare::to_vec(&proof).map_err(|_| EntityError::BareError.into())
    }

    pub(crate) fn verify_proof<V: ProfileVault>(
        channel_state: &[u8],
        responder_public_key: &PublicKey,
        proof: &[u8],
        vault: &mut V,
    ) -> ockam_core::Result<bool> {
        let proof: AuthenticationProof =
            serde_bare::from_slice(proof).map_err(|_| EntityError::BareError)?;

        vault.verify(proof.signature(), responder_public_key, channel_state)
    }
}

#[cfg(test)]
mod test {
    use crate::ProfileContacts;
    use crate::ProfileSecrets;
    use crate::{KeyAttributes, Profile};
    use crate::{ProfileAuth, ProfileImpl};
    use ockam_vault::SoftwareVault;
    use ockam_vault_sync_core::VaultMutex;
    use rand::prelude::*;

    #[test]
    fn authentication() {
        let vault = VaultMutex::create(SoftwareVault::default());

        let mut alice = ProfileImpl::create_internal(None, vault.clone()).unwrap();
        let mut bob = ProfileImpl::create_internal(None, vault.clone()).unwrap();

        // Secure channel is created here
        let mut key_agreement_hash = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut key_agreement_hash);

        // Network transfer: contact_alice, proof_alice -> B
        let contact_alice = alice.serialize_to_contact().unwrap();
        let proof_alice = alice
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Network transfer: contact_bob, proof_bob -> A
        let contact_bob = bob.serialize_to_contact().unwrap();
        let proof_bob = bob
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Alice&Bob add each other to contact list
        let contact_alice = Profile::deserialize_contact(contact_alice.as_slice()).unwrap();
        let alice_id = contact_alice.identifier().clone();
        bob.verify_and_add_contact(contact_alice).unwrap();
        let contact_bob = Profile::deserialize_contact(contact_bob.as_slice()).unwrap();
        let bob_id = contact_bob.identifier().clone();
        alice.verify_and_add_contact(contact_bob).unwrap();

        // If those calls succeed - we're good
        alice
            .verify_authentication_proof(&key_agreement_hash, &bob_id, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(&key_agreement_hash, &alice_id, proof_alice.as_slice())
            .unwrap();
    }

    #[test]
    fn authentication_profile_update_key_rotated() {
        let vault = VaultMutex::create(SoftwareVault::default());

        let mut alice = ProfileImpl::create_internal(None, vault.clone()).unwrap();
        let mut bob = ProfileImpl::create_internal(None, vault.clone()).unwrap();

        let root_key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());

        alice.rotate_key(root_key_attributes.clone(), None).unwrap();
        bob.rotate_key(root_key_attributes.clone(), None).unwrap();

        // Secure channel is created here
        let mut key_agreement_hash = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut key_agreement_hash);

        // Network transfer: contact_alice, proof_alice -> B
        let contact_alice = alice.serialize_to_contact().unwrap();
        let proof_alice = alice
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Network transfer: contact_bob, proof_bob -> A
        let contact_bob = bob.serialize_to_contact().unwrap();
        let proof_bob = bob
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Alice&Bob add each other to contact list
        let contact_alice = Profile::deserialize_contact(contact_alice.as_slice()).unwrap();
        let alice_id = contact_alice.identifier().clone();
        bob.verify_and_add_contact(contact_alice).unwrap();
        let contact_bob = Profile::deserialize_contact(contact_bob.as_slice()).unwrap();
        let bob_id = contact_bob.identifier().clone();
        alice.verify_and_add_contact(contact_bob).unwrap();

        // If those calls succeed - we're good
        alice
            .verify_authentication_proof(&key_agreement_hash, &bob_id, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(&key_agreement_hash, &alice_id, proof_alice.as_slice())
            .unwrap();
    }

    #[test]
    fn authentication_profile_update_key_rotated_after_first_handshake() {
        let vault = VaultMutex::create(SoftwareVault::default());

        let mut alice = ProfileImpl::create_internal(None, vault.clone()).unwrap();
        let mut bob = ProfileImpl::create_internal(None, vault.clone()).unwrap();

        let root_key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());

        // Secure channel is created here
        let mut key_agreement_hash = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut key_agreement_hash);

        // Network transfer: contact_alice, proof_alice -> B
        let contact_alice = alice.serialize_to_contact().unwrap();
        let proof_alice = alice
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Network transfer: contact_bob, proof_bob -> A
        let contact_bob = bob.serialize_to_contact().unwrap();
        let proof_bob = bob
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        // Alice&Bob add each other to contact list
        let contact_alice = Profile::deserialize_contact(contact_alice.as_slice()).unwrap();
        let alice_id = contact_alice.identifier().clone();
        bob.verify_and_add_contact(contact_alice).unwrap();
        let contact_bob = Profile::deserialize_contact(contact_bob.as_slice()).unwrap();
        let bob_id = contact_bob.identifier().clone();
        alice.verify_and_add_contact(contact_bob).unwrap();

        // If those calls succeed - we're good
        alice
            .verify_authentication_proof(&key_agreement_hash, &bob_id, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(&key_agreement_hash, &alice_id, proof_alice.as_slice())
            .unwrap();

        let alice_index = alice.change_history().as_ref().len();
        alice.rotate_key(root_key_attributes.clone(), None).unwrap();
        let alice_changes = &alice.change_history().as_ref()[alice_index..];
        let alice_changes = Profile::serialize_change_events(&alice_changes).unwrap();
        let bob_index = bob.change_history().as_ref().len();
        bob.rotate_key(root_key_attributes.clone(), None).unwrap();
        let bob_changes = &bob.change_history().as_ref()[bob_index..];
        let bob_changes = Profile::serialize_change_events(&bob_changes).unwrap();

        let alice_changes = Profile::deserialize_change_events(alice_changes.as_slice()).unwrap();
        bob.verify_and_update_contact(&alice_id, alice_changes)
            .unwrap();

        let bob_changes = Profile::deserialize_change_events(bob_changes.as_slice()).unwrap();
        alice
            .verify_and_update_contact(&bob_id, bob_changes)
            .unwrap();

        let proof_alice = alice
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        let proof_bob = bob
            .generate_authentication_proof(&key_agreement_hash)
            .unwrap();

        alice
            .verify_authentication_proof(&key_agreement_hash, &bob_id, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(&key_agreement_hash, &alice_id, proof_alice.as_slice())
            .unwrap();
    }
}
