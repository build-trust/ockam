use crate::SecretKey;
use bls12_381_plus::{G2Affine, G2Projective};
use group::Curve;
use serde::{Deserialize, Serialize};
use signature_core::lib::*;
use subtle::Choice;

/// The public key contains a generator point for each
/// message that is signed and two extra.
/// See section 4.2 in
/// <https://eprint.iacr.org/2015/525.pdf> and
/// <https://eprint.iacr.org/2017/1197.pdf>
///
/// `w` corresponds to m' in the paper to achieve
/// EUF-CMA security level.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PublicKey {
    pub(crate) w: G2Projective,
    pub(crate) x: G2Projective,
    #[serde(with = "VecSerializer")]
    pub(crate) y: Vec<G2Projective, 128>,
}

impl Default for PublicKey {
    fn default() -> Self {
        Self {
            w: G2Projective::identity(),
            x: G2Projective::identity(),
            y: Vec::new(),
        }
    }
}

impl From<&SecretKey> for PublicKey {
    fn from(sk: &SecretKey) -> Self {
        let w = G2Projective::generator() * sk.w;
        let x = G2Projective::generator() * sk.x;
        let mut y = Vec::new();
        for s_y in &sk.y {
            y.push(G2Projective::generator() * s_y)
                .expect("allocate more space");
        }
        Self { w, x, y }
    }
}

impl PublicKey {
    const POINT_SIZE: usize = 96;

    /// Check if this public key is valid
    pub fn is_valid(&self) -> Choice {
        let mut res = !self.w.is_identity();
        res &= !self.x.is_identity();
        for y in &self.y {
            res &= !y.is_identity();
        }
        res
    }

    /// Check if this public key is invalid
    pub fn is_invalid(&self) -> Choice {
        let mut res = self.w.is_identity();
        res |= self.x.is_identity();
        for y in &self.y {
            res |= y.is_identity();
        }
        res
    }

    /// Store the public key as a sequence of bytes
    /// Each scalar is compressed to big-endian format
    /// Needs (N + 2) * 96 space otherwise it will panic
    /// where N is the number of messages that can be signed
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        let mut offset = 0;
        let mut end = Self::POINT_SIZE;
        buffer[offset..end].copy_from_slice(&self.w.to_affine().to_compressed()[..]);

        offset = end;
        end += Self::POINT_SIZE;

        buffer[offset..end].copy_from_slice(&self.x.to_affine().to_compressed()[..]);

        offset = end;
        end += Self::POINT_SIZE;

        for y in &self.y {
            buffer[offset..end].copy_from_slice(&y.to_affine().to_compressed()[..]);
            offset = end;
            end += Self::POINT_SIZE;
        }
    }

    /// Convert a byte sequence into the public key
    /// Expected size is (N + 2) * 96 bytes
    /// where N is the number of messages that can be signed
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        // Length for w, x, and 1 y
        const MIN_SIZE: usize = PublicKey::POINT_SIZE * 3;

        let buffer = bytes.as_ref();
        if buffer.len() % Self::POINT_SIZE != 0 {
            return None;
        }
        if buffer.len() < MIN_SIZE {
            return None;
        }

        fn from_be_bytes(d: &[u8]) -> G2Projective {
            use core::convert::TryFrom;

            let t = <[u8; PublicKey::POINT_SIZE]>::try_from(d).expect("invalid length");
            G2Affine::from_compressed(&t)
                .map(G2Projective::from)
                .unwrap()
        }

        let y_cnt = (buffer.len() / Self::POINT_SIZE) - 2;
        let mut offset = 0;
        let mut end = Self::POINT_SIZE;
        let w = from_be_bytes(&buffer[offset..end]);
        offset = end;
        end += Self::POINT_SIZE;

        let x = from_be_bytes(&buffer[offset..end]);
        offset = end;
        end += Self::POINT_SIZE;

        let mut y = Vec::new();

        for _ in 0..y_cnt {
            if y.push(from_be_bytes(&buffer[offset..end])).is_err() {
                return None;
            }
            offset = end;
            end += Self::POINT_SIZE;
        }
        Some(Self { w, x, y })
    }
}
