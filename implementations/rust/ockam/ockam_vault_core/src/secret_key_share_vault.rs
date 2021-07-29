use crate::types::{PublicKey,  SecretKey};
use ockam_core::Result;
use rand_core::{CryptoRng, RngCore};

const PARTIAL_SIGNATURE_BYTES: usize = 49;

///MPC and threashold signing functionality
pub trait SecretKeyShareVault {

    /// Secret share this key by creating `N` shares where `T` are required
    /// to combine back into this secret
    fn split_secret<R: CryptoRng + RngCore, const T: usize, const N: usize>(
        &self, secret: &SecretKey, rng: &mut R
    ) -> Result<[SecretKey; N]>;
    /// Reconstruct a secret key from shares created from `split_secret`
    fn combine_shares<const T: usize, const N: usize>(
        &self,
        shares: &[SecretKey]) -> Result<SecretKey>;
   
    /// Reconstruct a signature from partial signatures created from `partial_sign`
    fn combine_signatures<const T: usize, const N: usize>(
        &self, 
        signatures: &[[u8; PARTIAL_SIGNATURE_BYTES]]
        ) -> Result<[u8; PARTIAL_SIGNATURE_BYTES -1 ]>;
   
    /// Create a new partial signature
    fn partial_sign<B: AsRef<[u8]>>(
        &self, 
        sk: &SecretKey, msg: &B) -> Result<[u8; PARTIAL_SIGNATURE_BYTES]>;
      
    /// Verify if the combined signatures from `combine_signatures` was signed  with `msg` with `pk`
    fn verify_signatures<B: AsRef<[u8]>>(
        &self, 
        signature: &[u8; PARTIAL_SIGNATURE_BYTES -1], 
        pk: &PublicKey, msg: &B) -> Result<bool>;

}