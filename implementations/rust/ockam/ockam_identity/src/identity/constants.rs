/// `Identity`-related constants
pub struct IdentityConstants;

impl IdentityConstants {
    /// Sha256 of that value is used as previous change id for the first change
    pub const BUILD_TRUST: &'static [u8] = "BUILD_TRUST".as_bytes();
    /// Change history key for AttributesStorage
    pub const CHANGE_HISTORY_KEY: &'static str = "CHANGE_HISTORY";
    /// Attributes key for AttributesStorage
    pub const ATTRIBUTES_KEY: &'static str = "ATTRIBUTES";
}
