use ockam_core::compat::string::String;

/// This trait needs to be implemented by structs which are used as keys in a key/value file
/// This constraint is necessary due to the persistence of values as JSON in the underlying
/// FileValueStorage
pub trait ToStringKey {
    /// Return a string representation to be used as a key in a JSON map
    fn to_string_key(&self) -> String;
}
