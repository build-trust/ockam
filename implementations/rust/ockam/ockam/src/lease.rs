#![deny(missing_docs)]

use ockam_core::compat::{string::String, vec::Vec};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// A lease for managing secrets
#[derive(Debug, Serialize, Deserialize)]
pub struct Lease<T>
where
    T: DeserializeOwned + Serialize,
{
    /// Unique identifier
    pub id: [u8; 16],
    /// Unix timestamp in seconds when issued
    pub issued: u64,
    /// Can the lease be renewed or not
    pub renewable: bool,
    /// Any tags that the issuer applied to this lease
    pub tags: Vec<String>,
    /// The value thats leased
    #[serde(serialize_with = "T::serialize", deserialize_with = "T::deserialize")]
    pub value: T,
}

#[test]
fn test_serialization() {
    let secret = [0xFFu8; 32];
    let lease = Lease {
        id: [0x33; 16],
        issued: 1613519081,
        renewable: true,
        tags: [String::from("can-write"), String::from("can-read")].to_vec(),
        value: secret,
    };

    let res = serde_json::to_string(&lease);
    assert!(res.is_ok());
    let pickeled = res.unwrap();
    let res = serde_json::from_str::<Lease<[u8; 32]>>(&pickeled);
    assert!(res.is_ok());
    let lease2 = res.unwrap();

    assert_eq!(lease.id, lease2.id);
    assert_eq!(lease.issued, lease2.issued);
    assert_eq!(lease.tags, lease2.tags);
    assert_eq!(lease.value, lease2.value);

    let res = serde_bare::to_vec(&lease);
    assert!(res.is_ok());
    let bare = res.unwrap();
    let res = serde_bare::from_slice::<Lease<[u8; 32]>>(&bare);
    assert!(res.is_ok());
    let lease2 = res.unwrap();

    assert_eq!(lease.id, lease2.id);
    assert_eq!(lease.issued, lease2.issued);
    assert_eq!(lease.tags, lease2.tags);
    assert_eq!(lease.value, lease2.value);
}
