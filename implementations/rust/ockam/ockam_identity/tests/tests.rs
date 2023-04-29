use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType};
use ockam_core::{Error, Result};
use ockam_identity::identities;
use ockam_node::Context;
use rand::{thread_rng, RngCore};

#[ockam_macros::test]
async fn test_auth_use_case(ctx: &mut Context) -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let identities_keys = identities.identities_keys();

    // Alice and Bob are distinct Entities.
    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    identities_repository.update_known_identity(&bob).await?;
    identities_repository.update_known_identity(&alice).await?;

    // Some state known to both parties. In Noise this would be a computed hash, for example.
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = identities_keys
        .create_signature(&alice, &state, None)
        .await?;
    let bob_proof = identities_keys.create_signature(&bob, &state, None).await?;

    let known_bob = identities_repository
        .get_identity(&bob.identifier())
        .await?
        .unwrap();
    if !identities_keys
        .verify_signature(&known_bob, &bob_proof, &state, None)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    let known_alice = identities_repository
        .get_identity(&alice.identifier())
        .await?
        .unwrap();
    if !identities_keys
        .verify_signature(&known_alice, &alice_proof, &state, None)
        .await?
    {
        return test_error("alice's proof was invalid");
    }

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_key_rotation(ctx: &mut Context) -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let identities_keys = identities.identities_keys();

    // Alice and Bob are distinct Entities.
    let mut alice = identities_creation.create_identity().await?;
    let mut bob = identities_creation.create_identity().await?;

    // Both identities rotate keys.
    identities_keys.rotate_root_key(&mut alice).await?;
    identities_keys.rotate_root_key(&mut bob).await?;

    identities_repository.update_known_identity(&bob).await?;
    identities_repository.update_known_identity(&alice).await?;

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_update_contact_and_reprove(ctx: &mut Context) -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let identities_keys = identities.identities_keys();

    // Alice and Bob are distinct Entities.
    let mut alice = identities_creation.create_identity().await?;
    let mut bob = identities_creation.create_identity().await?;

    identities_repository.update_known_identity(&bob).await?;
    identities_repository.update_known_identity(&alice).await?;

    identities_keys.rotate_root_key(&mut alice).await?;
    identities_keys.rotate_root_key(&mut bob).await?;

    identities_repository.update_known_identity(&bob).await?;
    identities_repository.update_known_identity(&alice).await?;

    // Re-Prove
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = identities_keys
        .create_signature(&alice, &state, None)
        .await?;
    let bob_proof = identities_keys.create_signature(&bob, &state, None).await?;

    let known_bob = identities_repository
        .get_identity(&bob.identifier())
        .await?
        .unwrap();
    if !identities_keys
        .verify_signature(&known_bob, &bob_proof, &state, None)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    let known_alice = identities_repository
        .get_identity(&alice.identifier())
        .await?
        .unwrap();
    if !identities_keys
        .verify_signature(&known_alice, &alice_proof, &state, None)
        .await?
    {
        return test_error("alice's proof was invalid");
    }

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn add_key(ctx: &mut Context) -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let identities_vault = identities.vault();
    let identities_keys = identities.identities_keys();
    let mut identity = identities_creation.create_identity().await?;

    let key = identities_vault
        .secret_generate(SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            32,
        ))
        .await?;

    identities_keys
        .add_key(&mut identity, "test".into(), &key)
        .await?;

    ctx.stop().await
}

fn test_error<S: Into<String>>(error: S) -> Result<()> {
    Err(Error::new_without_cause(Origin::Identity, Kind::Unknown).context("msg", error.into()))
}
