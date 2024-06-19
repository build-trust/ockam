use crate::alloc::string::ToString;
use crate::models::{Identifier, TimestampInSeconds};
use crate::utils::now;
use core::fmt::{Display, Formatter};
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::ToOwned;
use ockam_core::compat::string::String;
use ockam_core::compat::{collections::BTreeMap, vec::Vec};
use ockam_core::Result;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

/// An entry on the AuthenticatedIdentities table.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry {
    #[n(1)]
    #[serde(serialize_with = "serialize_attributes", deserialize_with = "deserialize_attributes")]
    attributes: BTreeMap<Vec<u8>, Vec<u8>>,
    #[n(2)] added_at: TimestampInSeconds,
    #[n(3)] expires_at: Option<TimestampInSeconds>,
    #[n(4)] attested_by: Option<Identifier>,
}

fn serialize_attributes<S>(
    attrs: &BTreeMap<Vec<u8>, Vec<u8>>,
    s: S,
) -> core::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = s.serialize_map(Some(attrs.len()))?;
    for (key, value) in attrs {
        map.serialize_entry(
            &String::from_utf8_lossy(key),
            &String::from_utf8_lossy(value),
        )?;
    }
    map.end()
}

fn deserialize_attributes<'de, D>(
    d: D,
) -> core::result::Result<BTreeMap<Vec<u8>, Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map = <BTreeMap<String, String>>::deserialize(d)?;
    let mut result = BTreeMap::new();
    for (key, value) in map {
        result.insert(key.as_bytes().to_vec(), value.as_bytes().to_vec());
    }
    Ok(result)
}

impl Display for AttributesEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let attributes = self.deserialized_key_value_attrs();
        f.debug_struct("AttributesEntry")
            .field("attributes", &attributes.join(","))
            .field("added_at", &self.added_at)
            .field(
                "expires_at",
                &self.expires_at.map_or("n/a".to_string(), |e| e.to_string()),
            )
            .field(
                "attested_by",
                &self
                    .attested_by
                    .clone()
                    .map_or("n/a".to_string(), |e| e.to_string()),
            )
            .finish()
    }
}

impl AttributesEntry {
    /// Constructor
    pub fn new(
        attrs: BTreeMap<Vec<u8>, Vec<u8>>,
        added_at: TimestampInSeconds,
        expires_at: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Self {
        Self {
            attributes: attrs,
            added_at,
            expires_at,
            attested_by,
        }
    }

    /// The entry attributes
    pub fn attrs(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attributes
    }

    /// The entry attributes as a list of key=value strings
    pub fn deserialized_key_value_attrs(&self) -> Vec<String> {
        let mut attributes = vec![];
        for (key, value) in self.attributes.clone() {
            if let (Ok(k), Ok(v)) = (String::from_utf8(key), String::from_utf8(value)) {
                attributes.push(format!("{k}={v}"));
            }
        }
        attributes
    }

    /// Expiration time for this entry
    pub fn expires_at(&self) -> Option<TimestampInSeconds> {
        self.expires_at
    }

    /// Date that the entry was added
    pub fn added_at(&self) -> TimestampInSeconds {
        self.added_at
    }

    /// Who attested this attributes for this identity identifier
    pub fn attested_by(&self) -> Option<Identifier> {
        self.attested_by.to_owned()
    }
}

impl AttributesEntry {
    /// Create an AttributesEntry with just one name/value pair
    pub fn single(
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
        expires_at: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Result<Self> {
        let attrs = BTreeMap::from([(attribute_name, attribute_value)]);
        Ok(Self {
            attributes: attrs,
            added_at: now()?,
            expires_at,
            attested_by,
        })
    }
}
