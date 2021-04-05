use bls::{PublicKey, SecretKey};
use bls12_381_plus::{ExpandMsgXmd, G1Affine, G1Projective};
use group::{Curve, Group};
use rand_core::{CryptoRng, RngCore};
use short_group_signatures_core::lib::*;
use typenum::marker_traits::NonZero;

/// The generators that are used to sign a vector of commitments for a BBS+ bls
/// These must be the same generators used by sign, verify, prove, and open
///
/// If the desire is to create these once and publish them with the public key
/// use MessageGenerators<N>::random(rng).
///
/// To generate these in a deterministic manner, use MessageGenerators<N>::from
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageGenerators<N>
where
    N: ArrayLength<G1Projective> + NonZero,
{
    /// Blinding factor generator
    pub(crate) h0: G1Projective,
    /// Generators for messages
    pub(crate) h: Vec<G1Projective, N>,
}

impl<N> Default for MessageGenerators<N>
where
    N: ArrayLength<G1Projective> + NonZero,
{
    fn default() -> Self {
        Self {
            h0: G1Projective::identity(),
            h: Vec::<G1Projective, N>::new(),
        }
    }
}

impl<N> From<&SecretKey> for MessageGenerators<N>
where
    N: ArrayLength<G1Projective> + NonZero,
{
    fn from(sk: &SecretKey) -> Self {
        let pk = PublicKey::from(sk);
        Self::from(pk)
    }
}

impl<N> From<PublicKey> for MessageGenerators<N>
where
    N: ArrayLength<G1Projective> + NonZero,
{
    fn from(pk: PublicKey) -> Self {
        const DST: &[u8] = b"BLS12381G1_XMD:BLAKE2B_SSWU_RO_BBS+_SIGNATURES:1_0_0";
        const DATA_SIZE: usize = 201;

        // Convert to a normal public key but deterministically derive all the generators
        // using the hash to curve algorithm BLS12381G1_XMD:SHA-256_SSWU_RO denoted as H2C
        // h_0 <- H2C(w || I2OSP(0, 4) || I2OSP(0, 1) || I2OSP(message_count, 4))
        // h_i <- H2C(w || I2OSP(i, 4) || I2OSP(0, 1) || I2OSP(message_count, 4))

        let count = N::to_u32().to_be_bytes();
        let mut data = [0u8; DATA_SIZE];
        data[..192].copy_from_slice(&pk.0.to_affine().to_uncompressed());
        data[197..201].copy_from_slice(&count);
        let h0 = G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(&data[..], DST);
        let mut h = Vec::<G1Projective, N>::new();
        for i in 1u32..=N::to_u32() {
            data[193..197].copy_from_slice(&i.to_be_bytes());
            h.push(G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(
                &data[..],
                DST,
            ))
            .unwrap();
        }
        Self { h0, h }
    }
}

impl<N> MessageGenerators<N>
where
    N: ArrayLength<G1Projective> + NonZero,
{
    /// Number of bytes needed to represent a message generator
    pub const GENERATOR_BYTES: usize = 48;

    /// Randomly create the Message Generators
    pub fn random(mut rng: impl RngCore + CryptoRng) -> Self {
        let h0 = G1Projective::random(&mut rng);
        let mut h = Vec::<G1Projective, N>::new();
        for _ in 0..N::to_usize() {
            h.push(G1Projective::random(&mut rng)).unwrap();
        }
        Self { h0, h }
    }

    /// Store the generators as a sequence of bytes
    /// Each point is compressed to big-endian format
    /// Needs (N + 1) * 48 space otherwise it will panic
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        buffer[0..48].copy_from_slice(&self.h0.to_affine().to_compressed());
        let mut offset = Self::GENERATOR_BYTES;
        let mut end = Self::GENERATOR_BYTES * 2;

        for i in 0..N::to_usize() {
            buffer[offset..end].copy_from_slice(&self.h[i].to_affine().to_compressed());
            offset += Self::GENERATOR_BYTES;
            end += Self::GENERATOR_BYTES;
        }
    }

    /// Convert a byte sequence into the message generators
    /// Expected size is (N + 1) * 48 bytes
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        use core::convert::TryFrom;

        macro_rules! slice_48 {
            ($d:expr, $b:expr, $e:expr) => {
                &<[u8; 48]>::try_from(&$d[$b..$e]).unwrap();
            };
        }

        let size = (N::to_usize() + 1) * Self::GENERATOR_BYTES;
        let buffer = bytes.as_ref();
        if buffer.len() < size || buffer.len() % Self::GENERATOR_BYTES != 0 {
            return None;
        }

        let h0 = G1Affine::from_compressed(slice_48!(buffer, 0, 48)).map(|p| G1Projective::from(p));
        if h0.is_none().unwrap_u8() == 1 {
            return None;
        }
        let mut offset = Self::GENERATOR_BYTES;
        let mut end = Self::GENERATOR_BYTES * 2;

        let mut h = Vec::<G1Projective, N>::new();
        for _ in 0..N::to_usize() {
            let p = G1Affine::from_compressed(slice_48!(buffer, offset, end))
                .map(|p| G1Projective::from(p));
            if p.is_none().unwrap_u8() == 1 {
                return None;
            }
            h.push(p.unwrap()).unwrap();
            offset += Self::GENERATOR_BYTES;
            end += Self::GENERATOR_BYTES;
        }

        Some(Self { h0: h0.unwrap(), h })
    }
}
