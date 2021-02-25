use crate::{OckamError, ProfileVault};
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
    pub(crate) fn generate_proof(
        channel_state: &[u8],
        secret: &Secret,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<Vec<u8>> {
        let signature = vault.sign(secret, channel_state)?;

        let proof = AuthenticationProof::new(signature);

        serde_bare::to_vec(&proof).map_err(|_| OckamError::BareError.into())
    }

    pub(crate) fn verify_proof(
        channel_state: &[u8],
        responder_public_key: &PublicKey,
        proof: &[u8],
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<()> {
        let proof: AuthenticationProof =
            serde_bare::from_slice(proof).map_err(|_| OckamError::BareError)?;

        vault.verify(
            &proof.signature(),
            responder_public_key.as_ref(),
            channel_state,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{KeyAttributes, Profile};
    use ockam_vault::SoftwareVault;
    use rand::prelude::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn authentication() {
        let vault = Arc::new(Mutex::new(SoftwareVault::default()));

        let mut alice = Profile::create(None, vault.clone()).unwrap();
        let mut bob = Profile::create(None, vault).unwrap();

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

        let contact_alice = alice
            .deserialize_and_verify_contact(contact_alice.as_slice())
            .unwrap();
        let contact_bob = bob
            .deserialize_and_verify_contact(contact_bob.as_slice())
            .unwrap();

        // If those calls succeed - we're good
        alice
            .verify_authentication_proof(&key_agreement_hash, &contact_bob, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(
            &key_agreement_hash,
            &contact_alice,
            proof_alice.as_slice(),
        )
        .unwrap();

        // Alice&Bob add each other to contact list
        alice.add_contact(contact_bob).unwrap();
        bob.add_contact(contact_alice).unwrap();
    }

    #[test]
    fn authentication_profile_update_key_rotated() {
        let vault = Arc::new(Mutex::new(SoftwareVault::default()));

        let mut alice = Profile::create(None, vault.clone()).unwrap();
        let mut bob = Profile::create(None, vault).unwrap();

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

        let contact_alice = alice
            .deserialize_and_verify_contact(contact_alice.as_slice())
            .unwrap();
        let contact_bob = bob
            .deserialize_and_verify_contact(contact_bob.as_slice())
            .unwrap();

        // If those calls succeed - we're good
        alice
            .verify_authentication_proof(&key_agreement_hash, &contact_bob, proof_bob.as_slice())
            .unwrap();
        bob.verify_authentication_proof(
            &key_agreement_hash,
            &contact_alice,
            proof_alice.as_slice(),
        )
        .unwrap();

        // Alice&Bob add each other to contact list
        alice.add_contact(contact_bob).unwrap();
        bob.add_contact(contact_alice).unwrap();
    }
}
