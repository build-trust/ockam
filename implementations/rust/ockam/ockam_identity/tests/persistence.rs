use ockam_core::{route, Address};
use ockam_identity::{
    secure_channels, DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
    SecureChannelListenerOptions, SecureChannelOptions, SecureChannelSqlxDatabase, SecureChannels,
};
use ockam_node::compat::futures::FutureExt;
use ockam_node::database::SqlxDatabase;
use ockam_node::{Context, NodeBuilder};
use ockam_vault::storage::SecretsSqlxDatabase;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;

#[ockam_macros::test]
async fn test_key_exchange_only(ctx: &mut Context) -> ockam_core::Result<()> {
    let secure_channels_alice = secure_channels().await?;
    let secure_channels_bob = secure_channels().await?;

    let alice = secure_channels_alice
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let bob = secure_channels_bob
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    let bob_options = SecureChannelListenerOptions::new().key_exchange_only();
    secure_channels_bob.create_secure_channel_listener(ctx, &bob, "bob_listener", bob_options)?;

    let alice_options = SecureChannelOptions::new().key_exchange_only();
    let alice_channel = secure_channels_alice
        .create_secure_channel(ctx, &alice, route!["bob_listener"], alice_options)
        .await?;

    ctx.sleep(Duration::from_millis(200)).await;

    let bob_channel = secure_channels_bob
        .secure_channel_registry()
        .get_channel_list()[0]
        .clone();

    let msg1_alice = vec![1u8; 32];
    let msg2_alice = vec![2u8; 32];

    let msg1_bob = vec![1u8; 32];
    let msg2_bob = vec![2u8; 32];

    let EncryptionResponse::Ok(encrypted_msg1_alice) = ctx
        .send_and_receive(
            route![alice_channel.encryptor_api_address().clone()],
            EncryptionRequest(msg1_alice.clone()),
        )
        .await?
    else {
        panic!()
    };
    let EncryptionResponse::Ok(encrypted_msg2_alice) = ctx
        .send_and_receive(
            route![alice_channel.encryptor_api_address().clone()],
            EncryptionRequest(msg2_alice.clone()),
        )
        .await?
    else {
        panic!()
    };
    let EncryptionResponse::Ok(encrypted_msg1_bob) = ctx
        .send_and_receive(
            route![bob_channel.encryptor_api_address().clone()],
            EncryptionRequest(msg1_bob.clone()),
        )
        .await?
    else {
        panic!()
    };
    let EncryptionResponse::Ok(encrypted_msg2_bob) = ctx
        .send_and_receive(
            route![bob_channel.encryptor_api_address().clone()],
            EncryptionRequest(msg2_bob.clone()),
        )
        .await?
    else {
        panic!()
    };

    let DecryptionResponse::Ok(decrypted_msg1_alice) = ctx
        .send_and_receive(
            route![bob_channel.decryptor_api_address().clone()],
            DecryptionRequest(encrypted_msg1_alice),
        )
        .await?
    else {
        panic!()
    };

    let DecryptionResponse::Ok(decrypted_msg2_alice) = ctx
        .send_and_receive(
            route![bob_channel.decryptor_api_address().clone()],
            DecryptionRequest(encrypted_msg2_alice),
        )
        .await?
    else {
        panic!()
    };

    let DecryptionResponse::Ok(decrypted_msg1_bob) = ctx
        .send_and_receive(
            route![alice_channel.decryptor_api_address().clone()],
            DecryptionRequest(encrypted_msg1_bob),
        )
        .await?
    else {
        panic!()
    };

    let DecryptionResponse::Ok(decrypted_msg2_bob) = ctx
        .send_and_receive(
            route![alice_channel.decryptor_api_address().clone()],
            DecryptionRequest(encrypted_msg2_bob),
        )
        .await?
    else {
        panic!()
    };

    assert_eq!(msg1_alice, decrypted_msg1_alice);
    assert_eq!(msg2_alice, decrypted_msg2_alice);
    assert_eq!(msg1_bob, decrypted_msg1_bob);
    assert_eq!(msg2_bob, decrypted_msg2_bob);

    Ok(())
}

#[test]
fn test_persistence() -> ockam_core::Result<()> {
    let (_db_file_alice, db_file_alice_path) = NamedTempFile::new().unwrap().keep().unwrap();
    let db_file_alice_path_clone = db_file_alice_path.clone();

    let (_db_file_bob, db_file_bob_path) = NamedTempFile::new().unwrap().keep().unwrap();
    let db_file_bob_path_clone = db_file_bob_path.clone();

    struct PassBetweenEnv {
        decryptor_api_address_alice: Address,
        decryptor_remote_address_alice: Address,
        decryptor_api_address_bob: Address,
        decryptor_remote_address_bob: Address,
        msg1_alice: Vec<u8>,
        msg2_alice: Vec<u8>,
        msg1_bob: Vec<u8>,
        msg2_bob: Vec<u8>,
        encrypted_msg1_alice: Vec<u8>,
        encrypted_msg2_alice: Vec<u8>,
        encrypted_msg1_bob: Vec<u8>,
        encrypted_msg2_bob: Vec<u8>,
    }

    let (ctx1, mut executor1) = NodeBuilder::new().build();
    let data = executor1
        .execute(async move {
            let data = std::panic::AssertUnwindSafe(async {
                let db_alice =
                    SqlxDatabase::create_sqlite(db_file_alice_path_clone.as_path()).await?;
                let secure_channel_repository_alice =
                    Arc::new(SecureChannelSqlxDatabase::new(db_alice.clone()));
                let secrets_repository_alice = Arc::new(SecretsSqlxDatabase::new(db_alice));
                let db_bob = SqlxDatabase::create_sqlite(db_file_bob_path_clone.as_path()).await?;
                let secure_channel_repository_bob =
                    Arc::new(SecureChannelSqlxDatabase::new(db_bob.clone()));
                let secrets_repository_bob = Arc::new(SecretsSqlxDatabase::new(db_bob));

                let secure_channels_alice = SecureChannels::builder()
                    .await?
                    .with_secure_channel_repository(secure_channel_repository_alice.clone())
                    .with_secrets_repository(secrets_repository_alice)
                    .build();
                let secure_channels_bob = SecureChannels::builder()
                    .await?
                    .with_secure_channel_repository(secure_channel_repository_bob.clone())
                    .with_secrets_repository(secrets_repository_bob)
                    .build();

                let alice = secure_channels_alice
                    .identities()
                    .identities_creation()
                    .create_identity()
                    .await?;
                let bob = secure_channels_bob
                    .identities()
                    .identities_creation()
                    .create_identity()
                    .await?;

                let bob_options = SecureChannelListenerOptions::new()
                    .key_exchange_only()
                    .persist()?;
                secure_channels_bob.create_secure_channel_listener(
                    &ctx1,
                    &bob,
                    "bob_listener",
                    bob_options,
                )?;

                let alice_options = SecureChannelOptions::new().key_exchange_only().persist()?;
                let alice_channel = secure_channels_alice
                    .create_secure_channel(&ctx1, &alice, route!["bob_listener"], alice_options)
                    .await?;

                ctx1.sleep(Duration::from_millis(200)).await;

                let bob_channel = secure_channels_bob
                    .secure_channel_registry()
                    .get_channel_list()[0]
                    .clone();

                let msg1_alice = vec![1u8; 32];
                let msg2_alice = vec![2u8; 32];

                let msg1_bob = vec![1u8; 32];
                let msg2_bob = vec![2u8; 32];

                let EncryptionResponse::Ok(encrypted_msg1_alice) = ctx1
                    .send_and_receive(
                        route![alice_channel.encryptor_api_address().clone()],
                        EncryptionRequest(msg1_alice.clone()),
                    )
                    .await?
                else {
                    panic!()
                };
                let EncryptionResponse::Ok(encrypted_msg2_alice) = ctx1
                    .send_and_receive(
                        route![alice_channel.encryptor_api_address().clone()],
                        EncryptionRequest(msg2_alice.clone()),
                    )
                    .await?
                else {
                    panic!()
                };
                let EncryptionResponse::Ok(encrypted_msg1_bob) = ctx1
                    .send_and_receive(
                        route![bob_channel.encryptor_api_address().clone()],
                        EncryptionRequest(msg1_bob.clone()),
                    )
                    .await?
                else {
                    panic!()
                };
                let EncryptionResponse::Ok(encrypted_msg2_bob) = ctx1
                    .send_and_receive(
                        route![bob_channel.encryptor_api_address().clone()],
                        EncryptionRequest(msg2_bob.clone()),
                    )
                    .await?
                else {
                    panic!()
                };

                let data = PassBetweenEnv {
                    decryptor_api_address_alice: alice_channel.decryptor_api_address().clone(),
                    decryptor_remote_address_alice: alice_channel
                        .decryptor_remote_address()
                        .clone(),
                    decryptor_api_address_bob: bob_channel.decryptor_api_address().clone(),
                    decryptor_remote_address_bob: bob_channel.decryptor_messaging_address().clone(),
                    msg1_alice,
                    msg2_alice,
                    msg1_bob,
                    msg2_bob,
                    encrypted_msg1_alice,
                    encrypted_msg2_alice,
                    encrypted_msg1_bob,
                    encrypted_msg2_bob,
                };

                Result::<PassBetweenEnv, ockam_core::Error>::Ok(data)
            })
            .catch_unwind()
            .await;

            ctx1.shutdown_node().await?;

            data.unwrap()
        })
        .unwrap()
        .unwrap();

    let (ctx2, mut executor2) = NodeBuilder::new().build();
    executor2
        .execute(async move {
            let res = std::panic::AssertUnwindSafe(async {
                let db_alice = SqlxDatabase::create_sqlite(db_file_alice_path.as_path()).await?;
                let secure_channel_repository_alice =
                    Arc::new(SecureChannelSqlxDatabase::new(db_alice.clone()));
                let secrets_repository_alice = Arc::new(SecretsSqlxDatabase::new(db_alice));
                let db_bob = SqlxDatabase::create_sqlite(db_file_bob_path.as_path()).await?;
                let secure_channel_repository_bob =
                    Arc::new(SecureChannelSqlxDatabase::new(db_bob.clone()));
                let secrets_repository_bob = Arc::new(SecretsSqlxDatabase::new(db_bob));

                let secure_channels_alice = SecureChannels::builder()
                    .await?
                    .with_secure_channel_repository(secure_channel_repository_alice.clone())
                    .with_secrets_repository(secrets_repository_alice)
                    .build();
                let secure_channels_bob = SecureChannels::builder()
                    .await?
                    .with_secure_channel_repository(secure_channel_repository_bob.clone())
                    .with_secrets_repository(secrets_repository_bob)
                    .build();

                secure_channels_alice
                    .start_persisted_secure_channel_decryptor(
                        &ctx2,
                        &data.decryptor_remote_address_alice,
                    )
                    .await?;

                secure_channels_bob
                    .start_persisted_secure_channel_decryptor(
                        &ctx2,
                        &data.decryptor_remote_address_bob,
                    )
                    .await?;

                let DecryptionResponse::Ok(decrypted_msg1_alice) = ctx2
                    .send_and_receive(
                        route![data.decryptor_api_address_bob.clone()],
                        DecryptionRequest(data.encrypted_msg1_alice),
                    )
                    .await?
                else {
                    panic!()
                };

                let DecryptionResponse::Ok(decrypted_msg2_alice) = ctx2
                    .send_and_receive(
                        route![data.decryptor_api_address_bob.clone()],
                        DecryptionRequest(data.encrypted_msg2_alice),
                    )
                    .await?
                else {
                    panic!()
                };

                let DecryptionResponse::Ok(decrypted_msg1_bob) = ctx2
                    .send_and_receive(
                        route![data.decryptor_api_address_alice.clone()],
                        DecryptionRequest(data.encrypted_msg1_bob),
                    )
                    .await?
                else {
                    panic!()
                };

                let DecryptionResponse::Ok(decrypted_msg2_bob) = ctx2
                    .send_and_receive(
                        route![data.decryptor_api_address_alice.clone()],
                        DecryptionRequest(data.encrypted_msg2_bob),
                    )
                    .await?
                else {
                    panic!()
                };

                assert_eq!(data.msg1_alice, decrypted_msg1_alice);
                assert_eq!(data.msg2_alice, decrypted_msg2_alice);
                assert_eq!(data.msg1_bob, decrypted_msg1_bob);
                assert_eq!(data.msg2_bob, decrypted_msg2_bob);

                ockam_core::Result::<()>::Ok(())
            })
            .catch_unwind()
            .await;

            ctx2.shutdown_node().await?;

            res.unwrap()
        })
        .unwrap()
        .unwrap();

    Ok(())
}
