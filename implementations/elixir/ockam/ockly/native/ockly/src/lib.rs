use std::{
    future::Future,
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use lazy_static::lazy_static;
use ockam_identity::{
    models::{CredentialSchemaIdentifier, PurposeKeyAttestation, PurposePublicKey},
    utils::AttributesBuilder,
    Identifier, Identities, Vault,
};
use ockam_vault::{
    EdDSACurve25519SecretKey, HandleToSecret, SigningKeyType, SigningSecret,
    SigningSecretKeyHandle, SoftwareVaultForSecureChannels, SoftwareVaultForSigning,
    X25519PublicKey, X25519SecretKey,
};
use ockam_vault_aws::{AwsKmsConfig, AwsSigningVault, InitialKeysDiscovery};
use rustler::{Atom, Binary, Env, Error, NewBinary, NifResult};
use std::clone::Clone;
use std::collections::HashMap;
use tokio::{runtime::Runtime, task};

lazy_static! {
    static ref RUNTIME: Arc<Runtime> = Arc::new(Runtime::new().unwrap());
    static ref IDENTITIES: RwLock<Option<Arc<Identities>>> = RwLock::new(None);
    static ref IDENTITY_MEMORY_VAULT: RwLock<Option<Arc<SoftwareVaultForSigning>>> =
        RwLock::new(None);
    static ref SECURE_CHANNEL_MEMORY_VAULT: RwLock<Option<Arc<SoftwareVaultForSecureChannels>>> =
        RwLock::new(None);
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
    invalid_secret_handle,
    invalid_public_key,
    no_memory_vault,
    aws_vault_loading_error,
    identities_ref_missing,
    secure_channel_vault_missing,
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

fn load(_env: rustler::Env, _load_data: rustler::Term) -> bool {
    load_memory_vault()
}

fn identities_ref() -> NifResult<Arc<Identities>> {
    let r = IDENTITIES
        .read()
        .map_err(|_| Error::Term(Box::new(atoms::identities_ref_missing())))?;
    r.clone()
        .ok_or_else(|| Error::Term(Box::new(atoms::invalid_state())))
}

fn load_memory_vault() -> bool {
    let identity_vault = SoftwareVaultForSigning::create();
    let secure_channel_vault = SoftwareVaultForSecureChannels::create();
    *IDENTITY_MEMORY_VAULT.write().unwrap() = Some(identity_vault.clone());
    *SECURE_CHANNEL_MEMORY_VAULT.write().unwrap() = Some(secure_channel_vault.clone());
    let builder = ockam_identity::Identities::builder().with_vault(Vault::new(
        identity_vault,
        secure_channel_vault,
        Vault::create_credential_vault(),
        Vault::create_verifying_vault(),
    ));
    *IDENTITIES.write().unwrap() = Some(builder.build());
    true
}

#[rustler::nif]
fn setup_aws_kms(key_ids: Vec<String>) -> NifResult<bool> {
    let secure_channel_vault = match SECURE_CHANNEL_MEMORY_VAULT.read().unwrap().clone() {
        Some(secure_channel_vault) => secure_channel_vault,
        None => return Err(Error::Term(Box::new(atoms::attestation_decode_error()))),
    };

    let key_ids = key_ids
        .into_iter()
        .map(|x| {
            SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(x.as_bytes().to_vec()))
        })
        .collect();
    block_future(async move {
        let config = AwsKmsConfig::default()
            .await
            .map_err(|e| Error::Term(Box::new(e.to_string())))?
            .with_initial_keys_discovery(InitialKeysDiscovery::Keys(key_ids));
        match AwsSigningVault::create_with_config(config).await {
            Ok(vault) => {
                let aws_vault = Arc::new(vault);
                let builder = ockam_identity::Identities::builder().with_vault(Vault::new(
                    aws_vault.clone(),
                    secure_channel_vault,
                    aws_vault,
                    Vault::create_verifying_vault(),
                ));
                *IDENTITIES.write().unwrap() = Some(builder.build());
                Ok(true)
            }
            Err(err) => Err(Error::Term(Box::new(err.to_string()))),
        }
    })
}

#[rustler::nif]
fn create_identity(env: Env, existing_key: Option<String>) -> NifResult<(Binary, Binary)> {
    let identities_ref = identities_ref()?;

    let (secret_type, existing_key) = if IDENTITY_MEMORY_VAULT.read().unwrap().is_some() {
        let existing_key = match existing_key {
            Some(handle) => {
                // Vault Handle
                let handle = hex::decode(handle)
                    .map_err(|_| Error::Term(Box::new(atoms::invalid_secret_handle())))?;

                Some(SigningSecretKeyHandle::EdDSACurve25519(
                    HandleToSecret::new(handle),
                ))
            }
            None => None,
        };
        (SigningKeyType::EdDSACurve25519, existing_key)
    } else {
        let existing_key = existing_key.map(|x| {
            // AWS KeyId
            SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(x.as_bytes().to_vec()))
        });
        (SigningKeyType::ECDSASHA256CurveP256, existing_key)
    };
    let identity = block_future(async move {
        let builder = identities_ref.identities_creation().identity_builder();

        let builder = match existing_key {
            Some(key) => builder.with_existing_key(key),

            None => builder.with_random_key(secret_type),
        };

        builder.build().await
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
    secret: Binary, // TODO: PublicKey is enough here
) -> NifResult<Binary<'a>> {
    let secure_channel_vault = match SECURE_CHANNEL_MEMORY_VAULT.read().unwrap().clone() {
        Some(secure_channel_vault) => secure_channel_vault,
        None => return Err(Error::Term(Box::new(atoms::secure_channel_vault_missing()))),
    };
    let identities_ref = identities_ref()?;
    let identifier = Identifier::from_str(&identifier)
        .map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let secret = secret
        .to_vec()
        .try_into()
        .map_err(|_| Error::Term(Box::new(atoms::invalid_secret())))?;
    let purpose_key = block_future(async move {
        let handle = secure_channel_vault
            .import_static_x25519_secret(X25519SecretKey::new(secret))
            .await?;
        identities_ref
            .purpose_keys()
            .purpose_keys_creation()
            .secure_channel_purpose_key_builder(&identifier)
            .with_existing_key(handle)
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
    let k = public_key
        .as_slice()
        .try_into()
        .map_err(|_| Error::Term(Box::new(atoms::invalid_public_key())))?;
    let k = X25519PublicKey(k);
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
                if let PurposePublicKey::SecureChannelStatic(x) = data.public_key {
                    if x == k {
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
        let mut attr_builder = AttributesBuilder::with_schema(CredentialSchemaIdentifier(0));
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
    let signing_vault = IDENTITY_MEMORY_VAULT
        .read()
        .unwrap()
        .clone()
        .ok_or_else(|| Error::Term(Box::new(atoms::no_memory_vault())))?;
    let secret = secret
        .to_vec()
        .try_into()
        .map_err(|_| Error::Term(Box::new(atoms::invalid_secret())))?;
    block_future(async move {
        let handle = signing_vault
            .import_key(SigningSecret::EdDSACurve25519(
                EdDSACurve25519SecretKey::new(secret),
            ))
            .await
            .map_err(|_| Error::Term(Box::new(atoms::invalid_secret())))?;

        Ok(hex::encode(handle.handle().value()))
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
