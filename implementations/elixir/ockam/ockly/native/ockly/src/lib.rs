use std::{sync::Arc, future::Future, str::FromStr, time::Duration, ops::Deref};

use rustler::{NifResult, Binary, NewBinary, Env, Error, Atom};
use tokio::{runtime::Runtime, task};
use lazy_static::lazy_static;
use std::clone::Clone;
use ockam_identity::{identities::identities, Identities, purpose_key::Purpose::SecureChannel, Identifier, models::{PurposeKeyAttestation, PurposePublicKey, SchemaId}, utils::AttributesBuilder};
use ockam_vault::PublicKey;
use ockam_vault::SecretType;
use std::collections::HashMap;

lazy_static! {
    static ref RUNTIME: Arc<Runtime> = Arc::new(Runtime::new().unwrap());
    static ref IDENTITIES: Arc<Identities> = identities();
}

mod atoms {
    rustler::atoms! {
	credential_decode_error,
    credential_encode_error,
    credential_issuing_error,
	identity_import_error,
	credential_verification_failed,
    invalid_identifier,
	utf8_error,
    attest_error,
    attestation_encode_error,
    attestation_decode_error,
    purpose_key_type_not_supported,
    invalid_attestation,
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


#[rustler::nif]
fn create_identity(env: Env) -> NifResult<(Binary, Binary)> {
    let identity = block_future(async move {
        IDENTITIES.identities_creation().create_identity().await
    }).map_err(|_| Error::BadArg)?;

    let exported = identity.export().map_err(|_| Error::BadArg)?;
    let id = identity.identifier().to_string();
    let mut binary = NewBinary::new(env, id.len());
    binary.copy_from_slice(id.as_bytes());
    let mut exp_binary = NewBinary::new(env, exported.len());
    exp_binary.copy_from_slice(&exported);
    Ok( (binary.into(), exp_binary.into()) )
}


#[rustler::nif]
fn attest_purpose_key<'a>(env: Env<'a>, identifier: String, public_key: Binary) -> NifResult<Binary<'a>> {
    let identifier = Identifier::from_str(&identifier).map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let k = PublicKey::new(public_key.as_slice().to_vec(), SecretType::X25519);
    let purpose_key = block_future(async move {
        IDENTITIES.purpose_keys().attest_purpose_key(&identifier, SecureChannel, k).await
    }).map_err(|_| Error::Term(Box::new(atoms::attest_error())))?;
    let encoded = minicbor::to_vec(purpose_key).map_err(|_| Error::Term(Box::new(atoms::attestation_encode_error())))?;
    let mut exp_binary = NewBinary::new(env, encoded.len());
    exp_binary.copy_from_slice(&encoded);
    Ok(exp_binary.into())
}

#[rustler::nif]
fn verify_purpose_key_attestation(identity: Binary, public_key: Binary,  attestation: Binary) -> NifResult<bool> {
    let attestation : PurposeKeyAttestation = minicbor::decode(&attestation).map_err(|_| Error::Term(Box::new(atoms::attestation_decode_error())))?;
    let k = PublicKey::new(public_key.as_slice().to_vec(), SecretType::X25519);
    block_future(async move {
        let identity = IDENTITIES.identities_creation().import(None, &identity).await.map_err(|_| atoms::identity_import_error())?;
        IDENTITIES.purpose_keys()
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
                        }})
        }).map_err(|reason| Error::Term(Box::new(reason)))
}

#[rustler::nif]
fn check_identity<'a>(env: Env<'a>, identity: Binary) -> NifResult<Binary<'a>> {
    let imported_identity = block_future(async move {
        IDENTITIES.identities_creation().import(None, &identity).await.map_err(|_| atoms::identity_import_error())
    }).map_err(|reason| Error::Term(Box::new(reason)))?;
    let id = imported_identity.identifier().to_string();
    let mut binary = NewBinary::new(env, id.len());
    binary.copy_from_slice(id.as_bytes());
    Ok(binary.into())
}


#[rustler::nif]
fn issue_credential<'a>(env: Env<'a>, issuer_identity: Binary,  subject_identifier: String, attrs: HashMap<String, String>, duration: u64) -> NifResult<Binary<'a>> {
    let subject_identifier = Identifier::from_str(&subject_identifier).map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let credential = block_future(async move {
        let issuer = IDENTITIES.identities_creation().import(None, &issuer_identity).await.map_err(|_| atoms::identity_import_error())?;
        let mut attr_builder = AttributesBuilder::with_schema(SchemaId(0));
        for (key, value) in attrs {
            attr_builder = attr_builder.with_attribute(key, value)
        }
        IDENTITIES.credentials().issue_credential(&issuer.identifier(), &subject_identifier, attr_builder.build(), Duration::from_secs(duration)).await.map_err(|_| atoms::credential_issuing_error())
    }).map_err(|reason| Error::Term(Box::new(reason)))?;
    let encoded = minicbor::to_vec(credential).map_err(|_| Error::Term(Box::new(atoms::credential_encode_error())))?;
    let mut binary = NewBinary::new(env, encoded.len());
    binary.copy_from_slice(&encoded);
    Ok(binary.into())
}

 
#[rustler::nif]
fn verify_credential<'a>(expected_subject: String, authorities: Vec<Binary>, credential: Binary) -> NifResult<(u64, HashMap<String, String>)> {
    let expected_subject = Identifier::from_str(&expected_subject).map_err(|_| Error::Term(Box::new(atoms::invalid_identifier())))?;
    let attributes = block_future(async move {
        let credential_and_purpose_key = minicbor::decode(&credential).map_err(|_| atoms::credential_decode_error())?;

        let mut authorities_identities = Vec::new();
        for authority in authorities {
            let authority = IDENTITIES.identities_creation().import(None, &authority).await.map_err(|_| atoms::identity_import_error())?;
            authorities_identities.push(authority.identifier().clone());
        }
        let credential_and_purpose_key_data = IDENTITIES.credentials().verify_credential(Some(&expected_subject), &authorities_identities, &credential_and_purpose_key).await.map_err(|_| atoms::credential_verification_failed())?;
        let mut attr_map = HashMap::new();
        for (k,v) in credential_and_purpose_key_data.credential_data.subject_attributes.map {
            attr_map.insert(String::from_utf8(k).map_err(|_| atoms::utf8_error())?, String::from_utf8(v).map_err(|_| atoms::utf8_error())?);
        }
        Ok((credential_and_purpose_key_data.credential_data.expires_at.deref().clone(), attr_map))
    });
    attributes.map_err(|reason : Atom| Error::Term(Box::new(reason)))
}


rustler::init!("Elixir.Ockly.Native", [create_identity, attest_purpose_key, verify_purpose_key_attestation, check_identity, issue_credential, verify_credential]);
