use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    #[serde(default = "VersionValue::latest")]
    pub version: VersionValue,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum VersionValue {
    #[serde(rename = "1")]
    V1,
}

impl VersionValue {
    pub fn latest() -> Self {
        VersionValue::V1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_config() {
        let config = "version: '1'";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::V1);
    }
}
