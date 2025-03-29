use crate::{LocalMultiaddrResolver, ReverseLocalConverter};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;

pub fn serialize_address_as_local_service<S>(
    address: &Address,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ma = ReverseLocalConverter::convert_address(address)
        .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
    serializer.serialize_str(&ma.to_string())
}

pub fn deserialize_address_from_local_service<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    let service_str = String::deserialize(deserializer)?;
    let multiaddr = MultiAddr::from_str(&service_str)
        .map_err(|e| serde::de::Error::custom(format!("Invalid MultiAddr: {}", e)))?;

    let route = LocalMultiaddrResolver::resolve(&multiaddr)
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;
    let address = route
        .next()
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;
    Ok(address.clone())
}

#[cfg(test)]
mod tests_serde_address_as_local_service {
    use super::*;
    use ockam_core::Address;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestStruct {
        #[serde(serialize_with = "serialize_address_as_local_service")]
        #[serde(deserialize_with = "deserialize_address_from_local_service")]
        address: Address,
    }

    #[test]
    fn test_serialize_address() {
        let address = Address::from_str("test_worker").unwrap();
        let test_struct = TestStruct { address };

        let serialized = serde_json::to_value(test_struct).unwrap();

        let expected_multiaddr = "/service/test_worker";
        assert_eq!(serialized["address"], json!(expected_multiaddr));
    }

    #[test]
    fn test_deserialize_address() {
        let json_value = json!({
            "address": "/service/test_worker"
        });

        let deserialized: TestStruct = serde_json::from_value(json_value).unwrap();

        assert_eq!(
            deserialized.address,
            Address::from_str("test_worker").unwrap()
        );
    }

    #[test]
    fn test_deserialize_invalid_multiaddr() {
        let json_value = json!({
            "address": "invalid-multiaddr"
        });

        let result: Result<TestStruct, _> = serde_json::from_value(json_value);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid MultiAddr"));
    }
}
