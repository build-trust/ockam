pub struct Version;

impl Version {
    /// Return the current crate version
    pub fn crate_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Return the hash of the current git commit
    pub fn git_hash() -> &'static str {
        env!("GIT_HASH")
    }
}
