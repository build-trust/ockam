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
        Self::Int(Self::latest_value())
    }

    fn latest_value() -> u8 {
        1
    }

    pub fn value(&self) -> u8 {
        let latest = Self::latest_value();
        let v = match self {
            Self::String(s) => s.parse().unwrap_or(latest),
            Self::Int(i) => *i,
        };
        if v < 1 || v > latest {
            latest
        } else {
            v
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
    fn value_as_string_or_int() {
        let config = "version: '1'";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::Int(1));

        let config = "version: 1";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::Int(1));
    }

    #[test]
    fn empty_defaults_to_latest() {
        let config = "";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::latest());
    }

    #[test]
    fn invalid_defaults_to_latest() {
        let config = "version: 0";
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::latest());

        let latest = VersionValue::latest_value();
        let config = &format!("version: {}", latest + 1);
        let parsed: Version = serde_yaml::from_str(config).unwrap();
        assert_eq!(parsed.version, VersionValue::latest());
    }
}
