use bls12_381_plus::{ExpandMsgXmd, G1Projective};
use core::convert::TryFrom;
use group::Curve;
use signature_bls::{PublicKey, SecretKey};
use signature_core::lib::*;

const DATA_SIZE: usize = 201;
const DST: &[u8] = b"BLS12381G1_XMD:BLAKE2B_SSWU_RO_BBS+_SIGNATURES:1_0_0";

/// The generators that are used to sign a vector of commitments for a BBS+ bls
/// These must be the same generators used by sign, verify, prove, and open
///
/// These are generated in a deterministic manner, use MessageGenerators::from_secret_key or
/// MessageGenerators::from_public_key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageGenerators {
    /// Blinding factor generator
    pub(crate) h0: G1Projective,
    length: usize,
    state: [u8; DATA_SIZE],
}

/// An iterator structure for generators
pub struct MessageGeneratorIter {
    index: usize,
    length: usize,
    state: [u8; DATA_SIZE],
}

impl Default for MessageGenerators {
    fn default() -> Self {
        Self {
            h0: G1Projective::identity(),
            length: 0,
            state: [0u8; DATA_SIZE],
        }
    }
}

impl Iterator for MessageGeneratorIter {
    type Item = G1Projective;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.length {
            return None;
        }
        self.index += 1;
        self.state[193..197].copy_from_slice(&(self.index as u32).to_be_bytes());
        Some(G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(
            &self.state[..],
            DST,
        ))
    }
}

impl MessageGenerators {
    /// Number of bytes needed to represent a message generator
    pub const GENERATOR_BYTES: usize = 48;

    /// The number of generators this object can generate
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns true when empty. Should not be possible. For completeness with `len` via clippy.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    /// Get the generator at `index`
    pub fn get(&self, index: usize) -> G1Projective {
        let mut state = self.state;
        state[193..197].copy_from_slice(&((index + 1) as u32).to_be_bytes());
        G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(&state[..], DST)
    }

    /// Create generators from the secret key
    pub fn from_secret_key(sk: &SecretKey, length: usize) -> Self {
        Self::from_public_key(PublicKey::from(sk), length)
    }

    /// Create generators from the public key
    pub fn from_public_key(pk: PublicKey, length: usize) -> Self {
        // Convert to a normal public key but deterministically derive all the generators
        // using the hash to curve algorithm BLS12381G1_XMD:SHA-256_SSWU_RO denoted as H2C
        // h_0 <- H2C(w || I2OSP(0, 4) || I2OSP(0, 1) || I2OSP(message_count, 4))
        // h_i <- H2C(w || I2OSP(i, 4) || I2OSP(0, 1) || I2OSP(message_count, 4))

        let count = (length as u32).to_be_bytes();
        let mut state = [0u8; DATA_SIZE];
        state[..192].copy_from_slice(&pk.0.to_affine().to_uncompressed());
        state[197..201].copy_from_slice(&count);

        let h0 = G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(&state[..], DST);

        Self { h0, length, state }
    }

    /// Store the internal state of this message generator
    pub fn to_bytes(&self) -> [u8; 205] {
        let data: [u8; 4] = (self.length as u32).to_be_bytes();
        let mut output = [0u8; 205];
        output[..4].copy_from_slice(&data);
        output[4..].copy_from_slice(&self.state[..]);
        output
    }

    /// Convert a sequence of bytes to a message generator
    pub fn from_bytes(bytes: &[u8; 205]) -> Self {
        let length = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[..4]).unwrap()) as usize;
        let mut state = [0u8; 201];
        state.copy_from_slice(&bytes[4..]);
        state[193] = 0;
        state[194] = 0;
        state[195] = 0;
        state[196] = 0;
        let h0 = G1Projective::hash::<ExpandMsgXmd<blake2::Blake2b>>(&state[..], DST);
        Self { h0, length, state }
    }

    /// Return an iterator over all the generators
    pub fn iter(&self) -> MessageGeneratorIter {
        MessageGeneratorIter {
            index: 0,
            state: self.state,
            length: self.length,
        }
    }
}
