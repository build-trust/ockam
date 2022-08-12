use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_core::{Error, Result};
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::Identity;
use ockam_node::Context;
use ockam_vault::Vault;
use rand::{thread_rng, RngCore};

fn test_error<S: Into<String>>(error: S) -> Result<()> {
    Err(Error::new_without_cause(Origin::Identity, Kind::Unknown).context("msg", error.into()))
}

#[ockam_macros::test]
async fn test_auth_use_case(ctx: &mut Context) -> Result<()> {
    let alice_vault = Vault::create();
    let bob_vault = Vault::create();

    let alice_storage = InMemoryStorage::new();
    let bob_storage = InMemoryStorage::new();

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    alice
        .update_known_identity(bob.identifier(), &bob.changes().await?, &alice_storage)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.changes().await?, &bob_storage)
        .await?;

    // Some state known to both parties. In Noise this would be a computed hash, for example.
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = alice.create_signature(&state).await?;
    let bob_proof = bob.create_signature(&state).await?;

    if !alice
        .verify_signature(&bob_proof, bob.identifier(), &state, &alice_storage)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    if !bob
        .verify_signature(&alice_proof, alice.identifier(), &state, &bob_storage)
        .await?
    {
        return test_error("alice's proof was invalid");
    }

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_key_rotation(ctx: &mut Context) -> Result<()> {
    let alice_vault = Vault::create();
    let bob_vault = Vault::create();

    let alice_storage = InMemoryStorage::new();
    let bob_storage = InMemoryStorage::new();

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    // Both identities rotate keys.
    alice.rotate_root_secret_key().await?;
    bob.rotate_root_secret_key().await?;

    alice
        .update_known_identity(bob.identifier(), &bob.changes().await?, &alice_storage)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.changes().await?, &bob_storage)
        .await?;

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_update_contact_and_reprove(ctx: &mut Context) -> Result<()> {
    let alice_vault = Vault::create();
    let bob_vault = Vault::create();

    let alice_storage = InMemoryStorage::new();
    let bob_storage = InMemoryStorage::new();

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    alice
        .update_known_identity(bob.identifier(), &bob.changes().await?, &alice_storage)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.changes().await?, &bob_storage)
        .await?;

    alice.rotate_root_secret_key().await?;
    bob.rotate_root_secret_key().await?;

    alice
        .update_known_identity(bob.identifier(), &bob.changes().await?, &alice_storage)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.changes().await?, &bob_storage)
        .await?;

    // Re-Prove
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = alice.create_signature(&state).await?;
    let bob_proof = bob.create_signature(&state).await?;

    if !alice
        .verify_signature(&bob_proof, bob.identifier(), &state, &alice_storage)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    if !bob
        .verify_signature(&alice_proof, alice.identifier(), &state, &bob_storage)
        .await?
    {
        return test_error("alice's proof was invalid");
    }

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn add_key(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();
    let e = Identity::create(ctx, &vault).await?;

    let key = vault
        .secret_generate(SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            32,
        ))
        .await?;

    e.add_key("test".into(), &key).await?;

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_basic_identity_key_ops(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let identity = Identity::create(ctx, &vault).await?;

    if !identity.verify_changes().await? {
        return test_error("verify_changes failed");
    }

    let secret1 = identity.get_root_secret_key().await?;
    let public1 = identity.get_root_public_key().await?;

    identity.create_key("Truck management".to_string()).await?;

    if !identity.verify_changes().await? {
        return test_error("verify_changes failed");
    }

    let secret2 = identity
        .get_secret_key("Truck management".to_string())
        .await?;
    let public2 = identity.get_public_key("Truck management".into()).await?;

    if secret1 == secret2 {
        return test_error("secret did not change after create_key");
    }

    if public1 == public2 {
        return test_error("public did not change after create_key");
    }

    identity.rotate_root_secret_key().await?;

    if !identity.verify_changes().await? {
        return test_error("verify_changes failed");
    }

    let secret3 = identity.get_root_secret_key().await?;
    let public3 = identity.get_root_public_key().await?;

    identity.rotate_root_secret_key().await?;

    if !identity.verify_changes().await? {
        return test_error("verify_changes failed");
    }

    if secret1 == secret3 {
        return test_error("secret did not change after rotate_key");
    }

    if public1 == public3 {
        return test_error("public did not change after rotate_key");
    }

    ctx.stop().await?;

    Ok(())
}
