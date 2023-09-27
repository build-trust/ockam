use std::{
    future::Future,
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use lazy_static::lazy_static;
use ockam_identity::{
    models::{PurposeKeyAttestation, PurposePublicKey, SchemaId},
    utils::AttributesBuilder,
    Identifier, Identities, Purpose, Vault,
};
use ockam_vault::SecretType;
use ockam_vault::{PublicKey, Secret, SoftwareSigningVault};
use ockam_vault_aws::{AwsSigningVault, AwsKmsConfig, InitialKeysDiscovery};
use rustler::{Atom, Binary, Env, Error, NewBinary, NifResult};
use std::clone::Clone;
use std::collections::HashMap;
use tokio::{runtime::Runtime, task};

lazy_static! {
    static ref RUNTIME: Arc<Runtime> = Arc::new(Runtime::new().unwrap());
    static ref IDENTITIES: RwLock<Option<Arc<Identities>>> = RwLock::new(None);
    static ref SIGNING_MEMORY_VAULT: RwLock<Option<Arc<SoftwareSigningVault>>> = RwLock::new(None);
}

mod atoms {
    rustler::atoms! {
    credential_decode_error,
    credential_encode_error,
    credential_issuing_error,
    identity_import_error,
    credential_verification_failed,
    invalid_identifier,
    identity_creation_error,
    identity_export_error,
    utf8_error,
    attest_error,
    attestation_encode_error,
    attestation_decode_error,
    purpose_key_type_not_supported,
    invalid_attestation,
    invalid_state,
    invalid_secret,
    no_memory_vault,
    aws_vault_loading_error,
    }
}

/// .
fn get_runtime() -> Arc<Runtime> {
    RUNTIME.clone()
}

fn block_future<F>(f: F) -> <F as Future>::Output
where
    F: Future,
{
    let rt = get_runtime();
    task::block_in_place(move || {
        let local = task::LocalSet::new();
        local.block_on(&rt, f)
    })
}

fn identities_ref() -> NifResult<Arc<Identities>> {
    let r = IDENTITIES.read().unwrap(); //TODO
    r.clone()
        .ok_or_else(|| Error::Term(Box::new(atoms::invalid_state())))
}

#[rustler::nif]
fn create_identity(env: Env, existing_key: Option<String>) -> NifResult<(Binary, Binary)> {
    let identities_ref = identities_ref()?;
    let secret_type = if SIGNING_MEMORY_VAULT.read().unwrap().is_some() {
        SecretType::Ed25519
    } else {
        SecretType::NistP256
    };
    let identity = block_future(async move {
        if let Some(key) = existing_key {
            identities_ref
                .identities_creation()
                .identity_builder()
                .with_existing_key(key, secret_type)
                .build()
                .await
        } else {
            identities_ref
                .identities_creation()
                .identity_builder()
                .with_random_key(secret_type)
                .build()
                .await
        }
    })
    .map_err(|_| Error::Term(Box::new(atoms::identity_creation_error())))?;

    let exported = identity
        .export()
        .map_err(|_| Error::Term(Box::new(atoms::identity_export_error())))?;
    let id = identity.identifier().to_string();
    let mut binary = NewBinary::new(env, id.len());
    binary.copy_from_slice(id.as_bytes());
    let mut exp_binary = NewBinary::new(env, exported.len());
    exp_binary.copy_from_slice(&exported);
    Ok((binary.into(), exp_binary.into()))
}

#[rustler::nif]
fn attest_secure_channel_key<'a>(
    env: Env<'a>,
    identifier: String,
    secret: Binary,
) -> NifResult<Binary<'a>> {
    let identities_ref = identities_ref()?;
    let identifier = Identifier::from_str(&identifier)
        .map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let purpose_key = block_future(async move {
        let key_id = identities_ref
            .purpose_keys()
            .purpose_keys_creation()
            .vault()
            .secure_channel_vault
            .import_static_secret(
                Secret::new(secret.to_vec()),
                ockam_vault::SecretAttributes::X25519,
            )
            .await?;
        identities_ref
            .purpose_keys()
            .purpose_keys_creation()
            .purpose_key_builder(&identifier, Purpose::SecureChannel)
            .with_existing_key(key_id, SecretType::X25519)
            .build()
            .await
    })
    .map_err(|_| Error::Term(Box::new(atoms::attest_error())))?;
    let encoded = minicbor::to_vec(purpose_key.attestation())
        .map_err(|_| Error::Term(Box::new(atoms::attestation_encode_error())))?;
    let mut exp_binary = NewBinary::new(env, encoded.len());
    exp_binary.copy_from_slice(&encoded);
    Ok(exp_binary.into())
}

#[rustler::nif]
fn verify_secure_channel_key_attestation(
    identity: Binary,
    public_key: Binary,
    attestation: Binary,
) -> NifResult<bool> {
    let identities_ref = identities_ref()?;
    let attestation: PurposeKeyAttestation = minicbor::decode(&attestation)
        .map_err(|_| Error::Term(Box::new(atoms::attestation_decode_error())))?;
    let k = PublicKey::new(public_key.as_slice().to_vec(), SecretType::X25519);
    block_future(async move {
        let identity = identities_ref
            .identities_creation()
            .import(None, &identity)
            .await
            .map_err(|_| atoms::identity_import_error())?;
        identities_ref
            .purpose_keys()
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(identity.identifier()), &attestation)
            .await
            .map_err(|_| atoms::attest_error())
            .and_then(|data| {
                if let PurposePublicKey::SecureChannelStaticKey(x) = data.public_key {
                    if PublicKey::from(x).eq(&k) {
                        Ok(true)
                    } else {
                        Err(atoms::invalid_attestation())
                    }
                } else {
                    Err(atoms::purpose_key_type_not_supported())
                }
            })
    })
    .map_err(|reason| Error::Term(Box::new(reason)))
}

#[rustler::nif]
fn check_identity<'a>(env: Env<'a>, identity: Binary) -> NifResult<Binary<'a>> {
    let identities_ref = identities_ref()?;
    let imported_identity = block_future(async move {
        identities_ref
            .identities_creation()
            .import(None, &identity)
            .await
            .map_err(|_| atoms::identity_import_error())
    })
    .map_err(|reason| Error::Term(Box::new(reason)))?;
    let id = imported_identity.identifier().to_string();
    let mut binary = NewBinary::new(env, id.len());
    binary.copy_from_slice(id.as_bytes());
    Ok(binary.into())
}

#[rustler::nif]
fn issue_credential<'a>(
    env: Env<'a>,
    issuer_identity: Binary,
    subject_identifier: String,
    attrs: HashMap<String, String>,
    duration: u64,
) -> NifResult<Binary<'a>> {
    let identities_ref = identities_ref()?;
    let subject_identifier = Identifier::from_str(&subject_identifier)
        .map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let credential_and_purpose_key = block_future(async move {
        let issuer = identities_ref
            .identities_creation()
            .import(None, &issuer_identity)
            .await
            .map_err(|_| atoms::identity_import_error())?;
        let mut attr_builder = AttributesBuilder::with_schema(SchemaId(0));
        for (key, value) in attrs {
            attr_builder = attr_builder.with_attribute(key, value)
        }
        identities_ref
            .credentials()
            .credentials_creation()
            .issue_credential(
                issuer.identifier(),
                &subject_identifier,
                attr_builder.build(),
                Duration::from_secs(duration),
            )
            .await
            .map_err(|_| atoms::credential_issuing_error())
    })
    .map_err(|reason| Error::Term(Box::new(reason)))?;
    let encoded = minicbor::to_vec(credential_and_purpose_key)
        .map_err(|_| Error::Term(Box::new(atoms::credential_encode_error())))?;
    let mut binary = NewBinary::new(env, encoded.len());
    binary.copy_from_slice(&encoded);
    Ok(binary.into())
}

#[rustler::nif]
fn verify_credential(
    expected_subject: String,
    authorities: Vec<Binary>,
    credential: Binary,
) -> NifResult<(u64, HashMap<String, String>)> {
    let identities_ref = identities_ref()?;
    let expected_subject = Identifier::from_str(&expected_subject)
        .map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let attributes = block_future(async move {
        let credential_and_purpose_key =
            minicbor::decode(&credential).map_err(|_| atoms::credential_decode_error())?;

        let mut authorities_identities = Vec::new();
        for authority in authorities {
            let authority = identities_ref
                .identities_creation()
                .import(None, &authority)
                .await
                .map_err(|_| atoms::identity_import_error())?;
            authorities_identities.push(authority.identifier().clone());
        }
        let credential_and_purpose_key_data = identities_ref
            .credentials()
            .credentials_verification()
            .verify_credential(
                Some(&expected_subject),
                &authorities_identities,
                &credential_and_purpose_key,
            )
            .await
            .map_err(|_| atoms::credential_verification_failed())?;
        let mut attr_map = HashMap::new();
        for (k, v) in credential_and_purpose_key_data
            .credential_data
            .subject_attributes
            .map
        {
            attr_map.insert(
                String::from_utf8(k.to_vec()).map_err(|_| atoms::utf8_error())?,
                String::from_utf8(v.to_vec()).map_err(|_| atoms::utf8_error())?,
            );
        }
        Ok((
            *credential_and_purpose_key_data
                .credential_data
                .expires_at
                .deref(),
            attr_map,
        ))
    });
    attributes.map_err(|reason: Atom| Error::Term(Box::new(reason)))
}

#[rustler::nif]
fn import_signing_secret(secret: Binary) -> NifResult<String> {
    let signing_vault = SIGNING_MEMORY_VAULT
        .read()
        .unwrap()
        .clone()
        .ok_or_else(|| Error::Term(Box::new(atoms::no_memory_vault())))?;
    block_future(async move {
        signing_vault
            .import_key(
                Secret::new(secret.to_vec()),
                ockam_vault::SecretAttributes::Ed25519,
            )
            .await
            .map_err(|_| Error::Term(Box::new(atoms::invalid_secret())))
    })
}

fn load_memory_vault() -> bool {
    let vault = SoftwareSigningVault::create();
    *SIGNING_MEMORY_VAULT.write().unwrap() = Some(vault.clone());
    let builder = ockam_identity::Identities::builder().with_vault(Vault::new(
        vault,
        Vault::create_secure_channel_vault(),
        Vault::create_credential_vault(),
        Vault::create_verifying_vault(),
    ));
    *IDENTITIES.write().unwrap() = Some(builder.build());
    true
}

fn load(_env: rustler::Env, _load_data: rustler::Term) -> bool {
    load_memory_vault()
}

#[rustler::nif]
fn setup_aws_kms(key_ids : Vec<String>) -> NifResult<bool> {
    block_future(async move {
        let config = AwsKmsConfig::default().await.map_err(|e| Error::Term(Box::new(e.to_string())))?
                                        .with_initial_keys_discovery(InitialKeysDiscovery::Keys(key_ids));
        match AwsSigningVault::create_with_config(config).await {
            Ok(vault) => {
                let aws_vault = Arc::new(vault);
                let builder = ockam_identity::Identities::builder().with_vault(Vault::new(
                    aws_vault.clone(),
                    Vault::create_secure_channel_vault(),
                    aws_vault,
                    Vault::create_verifying_vault(),
                ));
                *IDENTITIES.write().unwrap() = Some(builder.build());
                Ok(true)
            }
            Err(err) =>  Err(Error::Term(Box::new(err.to_string()))),
        }
    })
}

rustler::init!(
    "Elixir.Ockly.Native",
    [
        create_identity,
        attest_secure_channel_key,
        verify_secure_channel_key_attestation,
        check_identity,
        issue_credential,
        verify_credential,
        import_signing_secret,
        setup_aws_kms
    ],
    load = load
);
