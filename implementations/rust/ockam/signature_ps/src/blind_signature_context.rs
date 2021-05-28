use crate::SecretKey;
use blake2::VarBlake2b;
use bls12_381_plus::{G1Projective, Scalar};
use core::convert::TryFrom;
use digest::{Update, VariableOutput};
use group::{Curve, GroupEncoding};
use serde::{Deserialize, Serialize};
use signature_core::{constants::*, error::Error, lib::*};
use subtle::ConstantTimeEq;

/// Contains the data used for computing a blind signature and verifying
/// proof of hidden messages from a prover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindSignatureContext {
    /// The blinded signature commitment
    pub commitment: Commitment,
    /// The challenge hash for the Fiat-Shamir heuristic
    pub challenge: Challenge,
    #[serde(with = "VecSerializer")]
    /// The proofs for the hidden messages
    pub proofs: Vec<Challenge, 16>,
}

impl BlindSignatureContext {
    /// Store the generators as a sequence of bytes
    /// Each point is compressed to big-endian format
    /// Needs (N + 1) * 32 + 48 * 2 space otherwise it will panic
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        buffer[0..COMMITMENT_BYTES].copy_from_slice(&self.commitment.to_bytes());
        let mut offset = COMMITMENT_BYTES;
        let mut end = offset + FIELD_BYTES;

        buffer[offset..end].copy_from_slice(&self.challenge.to_bytes());

        offset = end;
        end += FIELD_BYTES;

        for i in 0..self.proofs.len() {
            buffer[offset..end].copy_from_slice(&self.proofs[i].to_bytes());
            offset = end;
            end += FIELD_BYTES;
        }
    }

    /// Convert a byte sequence into the blind signature context
    /// Expected size is (N + 1) * 32 + 48 bytes
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        let size = FIELD_BYTES * 2 + COMMITMENT_BYTES;
        let buffer = bytes.as_ref();
        if buffer.len() < size {
            return None;
        }
        if buffer.len() - COMMITMENT_BYTES % FIELD_BYTES != 0 {
            return None;
        }

        let commitment =
            Commitment::from_bytes(slicer!(buffer, 0, COMMITMENT_BYTES, COMMITMENT_BYTES));
        if commitment.is_none().unwrap_u8() == 1 {
            return None;
        }
        let mut offset = COMMITMENT_BYTES;
        let mut end = COMMITMENT_BYTES + FIELD_BYTES;

        let challenge = Challenge::from_bytes(slicer!(buffer, offset, end, FIELD_BYTES));
        if challenge.is_none().unwrap_u8() == 1 {
            return None;
        }

        let times = (buffer.len() - COMMITMENT_BYTES - FIELD_BYTES) / FIELD_BYTES;

        offset = end;
        end += FIELD_BYTES;

        let mut proofs = Vec::<Challenge, 16>::new();
        for _ in 0..times {
            let p = Challenge::from_bytes(slicer!(buffer, offset, end, FIELD_BYTES));
            if p.is_none().unwrap_u8() == 1 {
                return None;
            }
            proofs.push(p.unwrap()).unwrap();
            offset = end;
            end += FIELD_BYTES;
        }

        Some(Self {
            commitment: commitment.unwrap(),
            challenge: challenge.unwrap(),
            proofs,
        })
    }

    /// Assumes the proof of hidden messages
    /// If other proofs were included, those will need to be verified another way
    pub fn verify(
        &self,
        known_messages: &[usize],
        sk: &SecretKey,
        nonce: Nonce,
    ) -> Result<bool, Error> {
        let mut known = HashSet::new();
        let mut points = Vec::<G1Projective, 32>::new();
        for idx in known_messages {
            if *idx >= sk.y.len() {
                return Err(Error::new(1, "index out of bounds"));
            }
            known.insert(*idx);
        }
        for i in 0..sk.y.len() {
            if !known.contains(&i) {
                points
                    .push(G1Projective::generator() * sk.y[i])
                    .map_err(|_| Error::new((i + 1) as u32, ALLOC_MSG))?;
            }
        }
        points
            .push(G1Projective::generator())
            .map_err(|_| Error::new(sk.y.len() as u32, ALLOC_MSG))?;
        points
            .push(self.commitment.0)
            .map_err(|_| Error::new(sk.y.len() as u32, ALLOC_MSG))?;

        let mut scalars = self.proofs.iter().map(|p| p.0).collect::<Vec<Scalar, 32>>();
        scalars
            .push(self.challenge.0.neg())
            .map_err(|_| Error::new(sk.y.len() as u32, ALLOC_MSG))?;

        let mut res = [0u8; COMMITMENT_BYTES];
        let mut hasher = VarBlake2b::new(COMMITMENT_BYTES).unwrap();

        let commitment = G1Projective::sum_of_products_in_place(points.as_ref(), scalars.as_mut());
        hasher.update(&commitment.to_affine().to_bytes());
        hasher.update(&self.commitment.0.to_affine().to_uncompressed());
        hasher.update(nonce.to_bytes());
        hasher.finalize_variable(|out| {
            res.copy_from_slice(out);
        });
        let challenge = Scalar::from_okm(&res);

        Ok(self.challenge.0.ct_eq(&challenge).unwrap_u8() == 1)
    }
}
