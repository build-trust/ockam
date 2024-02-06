use ockam_core::env::FromString;

/// This struct can be used to parse environment variables representing boolean values
pub struct BooleanVar(pub String);

/// List of strings denoting a `false` value
const FALSE_VALUES: &[&str] = &["false", "FALSE", "NO", "no", "0"];

impl FromString for BooleanVar {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(BooleanVar(s.to_string()))
    }
}

impl BooleanVar {
    pub fn is_true(&self) -> bool {
        !FALSE_VALUES.contains(&self.0.as_str())
    }
}
