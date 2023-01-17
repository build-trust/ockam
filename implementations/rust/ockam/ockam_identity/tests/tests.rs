use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_core::{Error, Result};
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

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    alice
        .update_known_identity(bob.identifier(), &bob.to_public().await?)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.to_public().await?)
        .await?;

    // Some state known to both parties. In Noise this would be a computed hash, for example.
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = alice.create_signature(&state, None).await?;
    let bob_proof = bob.create_signature(&state, None).await?;

    let known_bob = alice.get_known_identity(bob.identifier()).await?.unwrap();
    if !known_bob
        .verify_signature(&bob_proof, &state, None, &alice_vault)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    let known_alice = bob.get_known_identity(alice.identifier()).await?.unwrap();
    if !known_alice
        .verify_signature(&alice_proof, &state, None, &bob_vault)
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

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    // Both identities rotate keys.
    alice.rotate_root_key().await?;
    bob.rotate_root_key().await?;

    alice
        .update_known_identity(bob.identifier(), &bob.to_public().await?)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.to_public().await?)
        .await?;

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_update_contact_and_reprove(ctx: &mut Context) -> Result<()> {
    let alice_vault = Vault::create();
    let bob_vault = Vault::create();

    // Alice and Bob are distinct Entities.
    let alice = Identity::create(ctx, &alice_vault).await?;
    let bob = Identity::create(ctx, &bob_vault).await?;

    alice
        .update_known_identity(bob.identifier(), &bob.to_public().await?)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.to_public().await?)
        .await?;

    alice.rotate_root_key().await?;
    bob.rotate_root_key().await?;

    alice
        .update_known_identity(bob.identifier(), &bob.to_public().await?)
        .await?;

    bob.update_known_identity(alice.identifier(), &alice.to_public().await?)
        .await?;

    // Re-Prove
    let state = {
        let mut state = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut state);
        state
    };

    let alice_proof = alice.create_signature(&state, None).await?;
    let bob_proof = bob.create_signature(&state, None).await?;

    let known_bob = alice.get_known_identity(bob.identifier()).await?.unwrap();
    if !known_bob
        .verify_signature(&bob_proof, &state, None, &alice_vault)
        .await?
    {
        return test_error("bob's proof was invalid");
    }

    let known_alice = bob.get_known_identity(alice.identifier()).await?.unwrap();
    if !known_alice
        .verify_signature(&alice_proof, &state, None, &bob_vault)
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
