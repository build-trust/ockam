use crate::{IdentityError, IdentityVault};
use ockam_core::compat::vec::Vec;
use ockam_core::vault::Signature;
use ockam_core::{Decodable, Encodable, Result};
use ockam_vault::{PublicKey, Secret};
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
    pub(crate) async fn generate_proof<V: IdentityVault>(
        channel_state: &[u8],
        secret: &Secret,
        vault: &mut V,
    ) -> Result<Vec<u8>> {
        let signature = vault.sign(secret, channel_state).await?;

        let proof = AuthenticationProof::new(signature);

        proof.encode().map_err(|_| IdentityError::BareError.into())
    }

    pub(crate) async fn verify_proof<V: IdentityVault>(
        channel_state: &[u8],
        responder_public_key: &PublicKey,
        proof: &[u8],
        vault: &mut V,
    ) -> Result<bool> {
        let proof = AuthenticationProof::decode(proof).map_err(|_| IdentityError::BareError)?;

        vault
            .verify(proof.signature(), responder_public_key, channel_state)
            .await
    }
}

#[cfg(test)]
mod test {

    use crate::{Identity, IdentityTrait};
    use ockam_core::{Error, Result};
    use ockam_node::Context;
    use ockam_vault::Vault;
    use rand::{thread_rng, RngCore};

    fn test_error<S: Into<String>>(error: S) -> Result<()> {
        Err(Error::new(0, error))
    }

    async fn test_auth_use_case(ctx: &Context) -> Result<()> {
        let alice_vault = Vault::create();
        let bob_vault = Vault::create();

        // Alice and Bob are distinct Entities.
        let mut alice = Identity::create(ctx, &alice_vault).await?;
        let mut bob = Identity::create(ctx, &bob_vault).await?;

        // Alice and Bob create Contacts
        let alice_contact = alice.as_contact().await?;
        let bob_contact = bob.as_contact().await?;

        // Alice and Bob exchange Contacts
        if !alice.verify_and_add_contact(bob_contact.clone()).await? {
            return test_error("alice failed to add bob");
        }

        if !bob.verify_and_add_contact(alice_contact.clone()).await? {
            return test_error("bob failed to add alice");
        }

        // Some state known to both parties. In Noise this would be a computed hash, for example.
        let state = {
            let mut state = [0u8; 32];
            let mut rng = thread_rng();
            rng.fill_bytes(&mut state);
            state
        };

        let alice_proof = alice.create_auth_proof(&state).await?;
        let bob_proof = bob.create_auth_proof(&state).await?;

        if !alice
            .verify_auth_proof(&state, bob_contact.identifier(), &bob_proof)
            .await?
        {
            return test_error("bob's proof was invalid");
        }

        if !bob
            .verify_auth_proof(&state, alice_contact.identifier(), &alice_proof)
            .await?
        {
            return test_error("alice's proof was invalid");
        }
        Ok(())
    }

    async fn test_key_rotation(ctx: &Context) -> Result<()> {
        let alice_vault = Vault::create();
        let bob_vault = Vault::create();

        // Alice and Bob are distinct Entities.
        let mut alice = Identity::create(ctx, &alice_vault).await?;
        let mut bob = Identity::create(ctx, &bob_vault).await?;

        // Both identitys rotate keys.
        alice.rotate_root_secret_key().await?;
        bob.rotate_root_secret_key().await?;

        // Alice and Bob create Contacts
        let alice_contact = alice.as_contact().await?;
        let bob_contact = bob.as_contact().await?;

        // Alice and Bob exchange Contacts. Verification still works with a rotation.
        if !alice.verify_and_add_contact(bob_contact.clone()).await? {
            return test_error("alice failed to add bob");
        }

        if !bob.verify_and_add_contact(alice_contact.clone()).await? {
            return test_error("bob failed to add alice");
        }

        Ok(())
    }

    async fn test_update_contact_and_reprove(ctx: &Context) -> Result<()> {
        let alice_vault = Vault::create();
        let bob_vault = Vault::create();

        let mut alice = Identity::create(ctx, &alice_vault).await?;
        let mut bob = Identity::create(ctx, &bob_vault).await?;

        // Alice and Bob create Contacts
        let alice_contact = alice.as_contact().await?;
        let bob_contact = bob.as_contact().await?;

        // Alice and Bob exchange Contacts
        if !alice.verify_and_add_contact(bob_contact.clone()).await? {
            return test_error("alice failed to add bob");
        }

        if !bob.verify_and_add_contact(alice_contact.clone()).await? {
            return test_error("bob failed to add alice");
        }

        // Some state known to both parties. In Noise this would be a computed hash, for example.
        let state = {
            let mut state = [0u8; 32];
            let mut rng = thread_rng();
            rng.fill_bytes(&mut state);
            state
        };

        let alice_proof = alice.create_auth_proof(&state).await?;
        let bob_proof = bob.create_auth_proof(&state).await?;

        if !alice
            .verify_auth_proof(&state, bob_contact.identifier(), &bob_proof)
            .await?
        {
            return test_error("bob's proof was invalid");
        }

        if !bob
            .verify_auth_proof(&state, alice_contact.identifier(), &alice_proof)
            .await?
        {
            return test_error("alice's proof was invalid");
        }

        alice.rotate_root_secret_key().await?;
        bob.rotate_root_secret_key().await?;

        let alice_contact = alice.as_contact().await?;
        let bob_contact = bob.as_contact().await?;

        // Copy Bob's last event (the rotation) and update Alice's view of Bob's Contact.
        let bob_last_event = bob_contact.change_events().last().unwrap().clone();
        if !alice
            .verify_and_update_contact(bob_contact.identifier(), &[bob_last_event])
            .await?
        {
            return test_error("alice failed to add bob");
        }

        // Copy Bob's last event (the rotation) and update Bob's view of Alice's Contact.
        let alice_last_event = alice_contact.change_events().last().unwrap().clone();
        if !bob
            .verify_and_update_contact(alice_contact.identifier(), &[alice_last_event])
            .await?
        {
            return test_error("bob failed to add alice");
        }

        // Re-Prove
        let state = {
            let mut state = [0u8; 32];
            let mut rng = thread_rng();
            rng.fill_bytes(&mut state);
            state
        };

        let alice_proof = alice.create_auth_proof(&state).await?;
        let bob_proof = bob.create_auth_proof(&state).await?;

        if !alice
            .verify_auth_proof(&state, bob_contact.identifier(), &bob_proof)
            .await?
        {
            return test_error("bob's proof was invalid");
        }

        if !bob
            .verify_auth_proof(&state, alice_contact.identifier(), &alice_proof)
            .await?
        {
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
