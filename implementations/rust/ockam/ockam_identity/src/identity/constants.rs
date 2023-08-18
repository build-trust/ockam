/// `Identity`-related constants
pub struct IdentityConstants;

impl IdentityConstants {
    /// Change history key for AttributesStorage
    pub const CHANGE_HISTORY_KEY: &'static str = "CHANGE_HISTORY";
    /// Key used to persist Secure Channel PurposeKey
    pub const SECURE_CHANNEL_PURPOSE_KEY: &'static str = "SC_PK";
    /// Key used to persist Credentials PurposeKey
    pub const CREDENTIALS_PURPOSE_KEY: &'static str = "C_PK";
    /// Attributes key for AttributesStorage
    pub const ATTRIBUTES_KEY: &'static str = "ATTRIBUTES";
}
