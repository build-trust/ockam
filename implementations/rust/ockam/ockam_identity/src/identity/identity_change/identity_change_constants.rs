/// `Identity`-related constants
pub struct IdentityChangeConstants;

impl IdentityChangeConstants {
    /// Sha256 of that value is used as previous change id for first change in a
    /// [`crate::SecureChannels`]
    pub const INITIAL_CHANGE: &'static [u8] = "OCKAM_INITIAL_CHANGE".as_bytes();
    /// Label for [`crate::SecureChannels`] update key
    pub const ROOT_LABEL: &'static str = "OCKAM_RK";
    /// Change history key for AttributesStorage
    pub const CHANGE_HISTORY_KEY: &'static str = "CHANGE_HISTORY";
    /// Attributes key for AttributesStorage
    pub const ATTRIBUTES_KEY: &'static str = "ATTRIBUTES";
}
