use crate::{EntityError, ProfileVault};
use ockam_core::compat::vec::Vec;
use ockam_vault::{PublicKey, Secret};
use ockam_vault_core::Signature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthenticationProof {
    signature: Signature,
}

impl AuthenticationProof {
    pub(crate) fn signature(&self) -> &Signature {
        &self.signature
    }
}

impl AuthenticationProof {
    pub(crate) fn new(signature: Signature) -> Self {
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

    use crate::{Entity, Identity};
    use ockam_core::Error;
    use ockam_node::Context;
    use ockam_vault_sync_core::Vault;
    use rand::{thread_rng, RngCore};

    fn test_error<S: Into<String>>(error: S) -> ockam_core::Result<()> {
        Err(Error::new(0, error))
    }

    async fn test_auth_use_case(ctx: &Context) -> ockam_core::Result<()> {
        let alice_vault = Vault::create(&ctx).expect("failed to create vault");
        let bob_vault = Vault::create(&ctx).expect("failed to create vault");

        // Alice and Bob are distinct Entities.
        let mut alice = Entity::create(&ctx, &alice_vault)?;
        let mut bob = Entity::create(&ctx, &bob_vault)?;

        // Alice and Bob create unique profiles for a Chat app.
        let mut alice_chat = alice.create_profile(&alice_vault)?;
        let mut bob_chat = bob.create_profile(&bob_vault)?;

        // Alice and Bob create Contacts
        let alice_contact = alice_chat.as_contact()?;
        let bob_contact = bob_chat.as_contact()?;

        // Alice and Bob exchange Contacts
        if !alice_chat.verify_and_add_contact(bob_contact.clone())? {
            return test_error("alice failed to add bob");
        }

        if !bob_chat.verify_and_add_contact(alice_contact.clone())? {
            return test_error("bob failed to add alice");
        }

        // Some state known to both parties. In Noise this would be a computed hash, for example.
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);

        let alice_proof = alice_chat.create_auth_proof(state)?;
        let bob_proof = bob_chat.create_auth_proof(state)?;

        if !alice_chat.verify_auth_proof(state, bob_contact.identifier(), bob_proof)? {
            return test_error("bob's proof was invalid");
        }

        if !bob_chat.verify_auth_proof(state, alice_contact.identifier(), alice_proof)? {
            return test_error("alice's proof was invalid");
        }
        Ok(())
    }

    async fn test_key_rotation(ctx: &Context) -> ockam_core::Result<()> {
        let alice_vault = Vault::create(&ctx).expect("failed to create vault");
        let bob_vault = Vault::create(&ctx).expect("failed to create vault");

        // Alice and Bob are distinct Entities.
        let mut alice = Entity::create(&ctx, &alice_vault)?;
        let mut bob = Entity::create(&ctx, &bob_vault)?;

        // Alice and Bob create unique profiles for a Chat app.
        let mut alice_chat = alice.create_profile(&alice_vault)?;
        let mut bob_chat = bob.create_profile(&bob_vault)?;

        // Both profiles rotate keys.
        alice_chat.rotate_profile_key()?;
        bob_chat.rotate_profile_key()?;

        // Alice and Bob create Contacts
        let alice_contact = alice_chat.as_contact()?;
        let bob_contact = bob_chat.as_contact()?;

        // Alice and Bob exchange Contacts. Verification still works with a rotation.
        if !alice_chat.verify_and_add_contact(bob_contact.clone())? {
            return test_error("alice failed to add bob");
        }

        if !bob_chat.verify_and_add_contact(alice_contact.clone())? {
            return test_error("bob failed to add alice");
        }

        Ok(())
    }

    async fn test_update_contact_and_reprove(ctx: &Context) -> ockam_core::Result<()> {
        let alice_vault = Vault::create(&ctx).expect("failed to create vault");
        let bob_vault = Vault::create(&ctx).expect("failed to create vault");

        let mut alice = Entity::create(&ctx, &alice_vault)?;
        let mut bob = Entity::create(&ctx, &bob_vault)?;

        // Alice and Bob create unique profiles for a Chat app.
        let mut alice_chat = alice.create_profile(&alice_vault)?;
        let mut bob_chat = bob.create_profile(&bob_vault)?;

        // Alice and Bob create Contacts
        let alice_contact = alice_chat.as_contact()?;
        let bob_contact = bob_chat.as_contact()?;

        // Alice and Bob exchange Contacts
        if !alice_chat.verify_and_add_contact(bob_contact.clone())? {
            return test_error("alice failed to add bob");
        }

        if !bob_chat.verify_and_add_contact(alice_contact.clone())? {
            return test_error("bob failed to add alice");
        }

        // Some state known to both parties. In Noise this would be a computed hash, for example.
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);

        let alice_proof = alice_chat.create_auth_proof(state)?;
        let bob_proof = bob_chat.create_auth_proof(state)?;

        if !alice_chat.verify_auth_proof(state, bob_contact.identifier(), bob_proof)? {
            return test_error("bob's proof was invalid");
        }

        if !bob_chat.verify_auth_proof(state, alice_contact.identifier(), alice_proof)? {
            return test_error("alice's proof was invalid");
        }

        alice_chat.rotate_profile_key()?;
        bob_chat.rotate_profile_key()?;

        let alice_contact = alice_chat.as_contact()?;
        let bob_contact = bob_chat.as_contact()?;

        // Copy Bob's last event (the rotation) and update Alice's view of Bob's Contact.
        let bob_last_event = bob_contact.change_events().last().unwrap().clone();
        if !alice_chat.verify_and_update_contact(bob_contact.identifier(), &[bob_last_event])? {
            return test_error("alice failed to add bob");
        }

        // Copy Bob's last event (the rotation) and update Bob's view of Alice's Contact.
        let alice_last_event = alice_contact.change_events().last().unwrap().clone();
        if !bob_chat.verify_and_update_contact(alice_contact.identifier(), &[alice_last_event])? {
            return test_error("bob failed to add alice");
        }

        // Re-Prove
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);

        let alice_proof = alice_chat.create_auth_proof(state)?;
        let bob_proof = bob_chat.create_auth_proof(state)?;

        if !alice_chat.verify_auth_proof(state, bob_contact.identifier(), bob_proof)? {
            return test_error("bob's proof was invalid");
        }

        if !bob_chat.verify_auth_proof(state, alice_contact.identifier(), alice_proof)? {
            return test_error("alice's proof was invalid");
        }

        Ok(())
    }

    #[test]
    fn authentication_tests() {
        let (mut ctx, mut exe) = ockam_node::start_node();
        exe.execute(async move {
            let mut results = Vec::new();

            // Individual Tests
            results.push(test_auth_use_case(&ctx).await);
            results.push(test_key_rotation(&ctx).await);
            results.push(test_update_contact_and_reprove(&ctx).await);

            // Stop before any assertions, or the panics are lost
            ctx.stop().await.unwrap();

            for r in results {
                match r {
                    Err(e) => panic!("test failure: {}", e),
                    _ => (),
                }
            }
        })
        .unwrap();
    }
}
