use crate::SecretKey;
use bls12_381_plus::{G1Affine, G1Projective};
use group::Curve;
use serde::{Deserialize, Serialize};
use signature_core::lib::*;
use subtle::Choice;

/// The generators contain a generator point for each
/// message that is blindly signed and two extra.
/// See section 4.2 in
/// <https://eprint.iacr.org/2015/525.pdf> and
/// <https://eprint.iacr.org/2017/1197.pdf>
///
/// `w` corresponds to m' in the paper to achieve
/// EUF-CMA security level.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageGenerators {
    #[serde(with = "VecSerializer")]
    pub(crate) y: Vec<G1Projective, 128>,
}

impl Default for MessageGenerators {
    fn default() -> Self {
        Self { y: Vec::new() }
    }
}

impl From<&SecretKey> for MessageGenerators {
    fn from(sk: &SecretKey) -> Self {
        let mut y = Vec::new();
        for s_y in &sk.y {
            y.push(G1Projective::generator() * s_y)
                .expect("allocate more space");
        }
        Self { y }
    }
}

impl MessageGenerators {
    const POINT_SIZE: usize = 48;

    /// Check if these generators are valid
    pub fn is_valid(&self) -> Choice {
        let mut res = Choice::from(1u8);
        for y in &self.y {
            res &= !y.is_identity();
        }
        res
    }

    /// Check if these generators are invalid
    pub fn is_invalid(&self) -> Choice {
        let mut res = Choice::from(0u8);
        for y in &self.y {
            res |= y.is_identity();
        }
        res
    }

    /// Store the generators as a sequence of bytes
    /// Each scalar is compressed to big-endian format
    /// Needs N * 48 space otherwise it will panic
    /// where N is the number of messages that can be signed
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        let mut offset = 0;
        let mut end = Self::POINT_SIZE;

        for y in &self.y {
            buffer[offset..end].copy_from_slice(&y.to_affine().to_compressed()[..]);
            offset = end;
            end += Self::POINT_SIZE;
        }
    }

    /// Convert a byte sequence into the public key
    /// Expected size is N * 48 bytes
    /// where N is the number of messages that can be signed
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        // Length for 1 y
        const MIN_SIZE: usize = MessageGenerators::POINT_SIZE;

        let buffer = bytes.as_ref();
        if buffer.len() % Self::POINT_SIZE != 0 {
            return None;
        }
        if buffer.len() < MIN_SIZE {
            return None;
        }

        fn from_be_bytes(d: &[u8]) -> G1Projective {
            use core::convert::TryFrom;

            let t = <[u8; MessageGenerators::POINT_SIZE]>::try_from(d).expect("invalid length");
            G1Affine::from_compressed(&t)
                .map(G1Projective::from)
                .unwrap()
        }

        let y_cnt = buffer.len() / Self::POINT_SIZE;
        let mut offset = 0;
        let mut end = Self::POINT_SIZE;
        let mut y = Vec::new();

        for _ in 0..y_cnt {
            if y.push(from_be_bytes(&buffer[offset..end])).is_err() {
                return None;
            }
            offset = end;
            end += Self::POINT_SIZE;
        }
        Some(Self { y })
    }
}
