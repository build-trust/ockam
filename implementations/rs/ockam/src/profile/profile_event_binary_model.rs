use crate::profile::profile::ProfileEventAttributes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProfileEventBinaryModel {
    version: u8,
    public_key: Option<Vec<u8>>,
    attributes: ProfileEventAttributes,
    prev_event_id: Option<String>,
    next_event_id: Option<String>,
}

impl ProfileEventBinaryModel {
    pub(crate) fn new(
        version: u8,
        public_key: Option<Vec<u8>>,
        attributes: ProfileEventAttributes,
        prev_event_id: Option<String>,
        next_event_id: Option<String>,
    ) -> Self {
        ProfileEventBinaryModel {
            version,
            public_key,
            attributes,
            prev_event_id,
            next_event_id,
        }
    }
}
