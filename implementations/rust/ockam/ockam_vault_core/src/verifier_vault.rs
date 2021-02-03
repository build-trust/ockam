use zeroize::Zeroize;

/// Signature verification vault functionality
pub trait VerifierVault: Zeroize {
    /// Verify a signature for given data using given public key
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &[u8],
        data: &[u8],
    ) -> ockam_core::Result<()>;
}
