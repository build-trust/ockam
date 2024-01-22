use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    #[serde(default = "VersionValue::latest")]
    pub version: VersionValue,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VersionValue {
    String(String),
    Int(u8),
}

impl VersionValue {
    pub fn latest() -> Self {
        VersionValue::Int(1)
    }

    pub fn value(&self) -> u8 {
        match self {
            VersionValue::String(s) => s.parse().unwrap_or(Self::latest().value()),
            VersionValue::Int(i) => *i,
        }
    }
}

impl PartialEq for VersionValue {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_config() {
        let config = "version: '1'";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::Int(1));

        let config = "version: 1";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::Int(1));
    }
}
