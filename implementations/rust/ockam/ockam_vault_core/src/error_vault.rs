use zeroize::Zeroize;

/// A vault that is aware of error domains.
pub trait ErrorVault: Zeroize {
    /// Returns the appropriate error domain of this vault.
    fn error_domain(&self) -> &'static str;
}
