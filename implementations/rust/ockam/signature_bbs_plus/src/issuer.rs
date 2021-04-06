use crate::{BlindSignature, BlindSignatureContext, Message, MessageGenerators, Nonce, Signature};
use bls::{PublicKey, SecretKey};
use bls12_381_plus::G1Projective;
use rand_core::{CryptoRng, RngCore};
use short_group_signatures_core::error::Error;
use short_group_signatures_core::lib::*;
use typenum::NonZero;

/// This struct represents an Issuer of signatures or Signer.
/// Provided are methods for signing regularly where all messages are known
/// and 2PC where some are only known to the holder and a blind signature
/// is created.
///
/// The issuer generates keys and uses those to sign
/// credentials. There are two types of public keys:
/// to the secret key. `DeterministicPublicKey` can be converted to a
/// `PublicKey` later. The latter is primarily used for storing a shorter
/// key and looks just like a regular ECC key.
pub struct Issuer;

impl Issuer {
    /// Create a keypair
    pub fn new_keys(rng: impl RngCore + CryptoRng) -> Result<(PublicKey, SecretKey), Error> {
        SecretKey::random(rng)
            .map(|sk| {
                let pk = PublicKey::from(&sk);
                (pk, sk)
            })
            .ok_or_else(|| Error::new(1, "invalid length to generate keys"))
    }

    /// Create a signature with no hidden messages
    pub fn sign<N, M>(
        sk: &SecretKey,
        generators: &MessageGenerators<N>,
        msgs: M,
    ) -> Result<Signature, Error>
    where
        N: ArrayLength<G1Projective> + NonZero,
        M: AsRef<[Message]>,
    {
        Signature::new(sk, generators, msgs)
    }

    /// Verify a proof of committed messages and generate a blind signature
    pub fn blind_sign<N>(
        ctx: &BlindSignatureContext,
        sk: &SecretKey,
        generators: &MessageGenerators<N>,
        msgs: &[(usize, Message)],
        nonce: Nonce,
    ) -> Result<BlindSignature, Error>
    where
        N: ArrayLength<G1Projective> + NonZero,
    {
        // Known messages are less than total, max at 128
        let tv1 = msgs.iter().map(|(i, _)| *i).collect::<Vec<usize, U128>>();
        if ctx.verify(tv1.as_ref(), generators, nonce)? {
            BlindSignature::new(ctx.commitment, sk, generators, msgs)
        } else {
            Err(Error::new(1, "invalid proof of committed messages"))
        }
    }

    /// Create a nonce used for the blind signing context
    pub fn generate_signing_nonce(rng: impl RngCore + CryptoRng) -> Nonce {
        Nonce::random(rng)
    }
}
