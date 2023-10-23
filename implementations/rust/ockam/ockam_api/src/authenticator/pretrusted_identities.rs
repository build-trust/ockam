use ockam::identity::utils::now;
use ockam::identity::{AttributesEntry, Identifier};
use ockam_core::compat::{collections::HashMap, string::String};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreTrustedIdentities {
    map: HashMap<Identifier, AttributesEntry>,
}

impl PreTrustedIdentities {
    pub fn new(map: HashMap<Identifier, AttributesEntry>) -> Self {
        Self { map }
    }

    pub fn new_from_raw_map(raw_map: HashMap<Identifier, HashMap<String, String>>) -> Result<Self> {
        let now = now()?;
        let map = raw_map
            .into_iter()
            .map(|(identity_id, raw_attrs)| {
                let attrs = raw_attrs
                    .into_iter()
                    .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
                    .collect();
                (identity_id, AttributesEntry::new(attrs, now, None, None))
            })
            .collect();

        let res = Self::new(map);

        Ok(res)
    }

    pub fn map(&self) -> &HashMap<Identifier, AttributesEntry> {
        &self.map
    }
}
