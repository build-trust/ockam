use crate::profile::change_event::Change;
use crate::profile::EventId;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureType {
    SelfSign,
    Previous,
}

#[derive(Debug, Clone)]
struct SignatureData([u8; 64]);

impl Serialize for SignatureData {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

// FIXME
struct SignatureDataVisitor;

impl<'de> Visitor<'de> for SignatureDataVisitor {
    type Value = SignatureData;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a 64-byte array")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v.len() == 64 {
            Ok(SignatureData(array_ref!(v, 0, 64).clone()))
        } else {
            Err(E::custom("Invalid signature bytes"))
        }
    }
}

impl<'de> Deserialize<'de> for SignatureData {
    fn deserialize<D>(deserializer: D) -> Result<SignatureData, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(SignatureDataVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Proof {
    Signature(Signature),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    stype: SignatureType, // Replace with enum
    data: SignatureData,
}

impl Signature {
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    pub fn data(&self) -> &[u8; 64] {
        &self.data.0
    }
}

impl Signature {
    pub fn new(stype: SignatureType, data: [u8; 64]) -> Self {
        Signature {
            stype,
            data: SignatureData(data),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Changes(Vec<Change>);

impl AsRef<[Change]> for Changes {
    fn as_ref(&self) -> &[Change] {
        &self.0
    }
}

impl Changes {
    pub fn new(changes: Vec<Change>) -> Self {
        Self(changes)
    }

    pub fn new_single(change: Change) -> Self {
        Self::new(vec![change])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedChangeEvent {
    version: u8,
    identifier: EventId,
    binary: Vec<u8>, // May be removed if needed, but may be useful
    changes: Changes,
    proofs: Vec<Proof>,
}

impl SignedChangeEvent {
    pub(crate) fn new(
        version: u8,
        identifier: EventId,
        binary: Vec<u8>,
        changes: Changes,
        proofs: Vec<Proof>,
    ) -> Self {
        SignedChangeEvent {
            version,
            identifier,
            binary,
            changes,
            proofs,
        }
    }
}

impl SignedChangeEvent {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn identifier(&self) -> &EventId {
        &self.identifier
    }
    pub fn binary(&self) -> &[u8] {
        &self.binary
    }
    pub fn changes(&self) -> &Changes {
        &self.changes
    }
    pub fn proofs(&self) -> &[Proof] {
        &self.proofs
    }
}
