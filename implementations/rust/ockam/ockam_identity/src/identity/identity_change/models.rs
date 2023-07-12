use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, string::String, vec::Vec};

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct IdentityIdModel(#[n(0)] Vec<u8>);

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct ChangeIdModel(#[n(0)] Vec<u8>);

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct PurposeKeyIdModel(#[n(0)] Vec<u8>);

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct IdentityModel {
    #[n(1)] changes: Vec<IdentityChangeModel>,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct IdentityChangeModel {
    #[n(1)] change_data: Vec<u8>,
    #[n(2)] self_signature: Vec<u8>,
    #[n(3)] prev_signature: Option<Vec<u8>>,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ChangeDataModel {
    #[n(1)] prev_change_id: Vec<u8>,
    #[n(2)] public_key: Vec<u8>,
    #[n(3)] key_type: KeyTypeModel,
    #[n(4)] creation_date: TimestampModel,
    #[n(5)] expiration_date: TimestampModel,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum KeyTypeModel {
    #[n(1)] Ed25519,
    #[n(2)] Curve25519,
    #[n(3)] P256,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyModel {
    #[n(1)] data: Vec<u8>,
    #[n(2)] signature: Vec<u8>,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyDataModel {
    #[n(1)] purpose: PurposeModel,
    #[n(2)] public_key: Vec<u8>,
    #[n(3)] key_type: KeyTypeModel,
    #[n(4)] issuer_identity_id: IdentityIdModel,
    #[n(5)] issuer_last_known_change_id: ChangeIdModel,
    #[n(6)] creation_date: TimestampModel,
    #[n(7)] expiration_date: TimestampModel,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum PurposeModel {
    #[n(1)] SecureChannelStaticKey,
    #[n(2)] CredentialsSigningKey,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct TimestampModel(#[n(0)] u64 /* seconds */ );

#[derive(Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    #[n(1)] credential_data: Vec<u8>,
    #[n(2)] credential_signature: Vec<u8>,
    #[n(3)] purpose_key: PurposeKeyModel,
}

#[derive(Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData {
    #[n(1)]  schema: SchemaModel,
    #[n(2)]  issuer_identity_id: IdentityIdModel,
    #[n(3)]  issuer_last_known_change_id: ChangeIdModel,
    #[n(5)]  subject_identity_id: IdentityIdModel,
    #[n(6)]  subject_last_known_change_id: ChangeIdModel,
    #[n(7)]  signer: PurposeKeyIdModel,
    #[n(8)]  attributes: AttributesModel,
    #[n(9)]  creation_date: TimestampModel,
    #[n(10)] expiration_date: TimestampModel,
}

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct SchemaModel(#[n(0)] u64);

#[derive(Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesModel {
    #[n(1)] attrs: BTreeMap<String, String>,
}
