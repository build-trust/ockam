use ockam::identity::utils::now;
use ockam::identity::{AttributesEntry, Identifier};
use ockam_core::compat::{collections::HashMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use serde::{Deserialize, Serialize};
use serde_json as json;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreTrustedIdentities {
    map: HashMap<Identifier, AttributesEntry>,
}

impl PreTrustedIdentities {
    pub fn new_from_string(entries: &str) -> Result<Self> {
        Ok(Self::new(Self::parse(entries)?))
    }

    pub fn new(map: HashMap<Identifier, AttributesEntry>) -> Self {
        Self { map }
    }

    fn parse(entries: &str) -> Result<HashMap<Identifier, AttributesEntry>> {
        let raw_map = json::from_str::<HashMap<Identifier, HashMap<String, String>>>(entries)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Invalid, e))?;
        let now = now()?;
        Ok(raw_map
            .into_iter()
            .map(|(identity_id, raw_attrs)| {
                let attrs = raw_attrs
                    .into_iter()
                    .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
                    .collect();
                (identity_id, AttributesEntry::new(attrs, now, None, None))
            })
            .collect())
    }
    pub fn map(&self) -> &HashMap<Identifier, AttributesEntry> {
        &self.map
    }
}

impl From<HashMap<Identifier, AttributesEntry>> for PreTrustedIdentities {
    fn from(map: HashMap<Identifier, AttributesEntry>) -> PreTrustedIdentities {
        PreTrustedIdentities::new(map)
    }
}
