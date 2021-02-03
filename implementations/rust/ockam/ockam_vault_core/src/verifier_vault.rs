use zeroize::Zeroize;

/// Trait with verify functionality
pub trait VerifierVault: Zeroize {
    /// Verify a signature
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &[u8],
        data: &[u8],
    ) -> ockam_core::Result<()>;
}
