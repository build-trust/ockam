use crate::PublicKey;
use bls12_381_plus::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar,
};
use core::convert::TryFrom;
use core::ops::BitOr;
use digest::Update;
use group::{Curve, Group, GroupEncoding};
use serde::{Deserialize, Serialize};
use signature_core::{constants::*, lib::*};

/// The actual proof that is sent from prover to verifier.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PokSignatureProof {
    pub(crate) sigma_1: G1Projective,
    pub(crate) sigma_2: G1Projective,
    pub(crate) commitment: G2Projective,
    #[serde(with = "VecSerializer")]
    pub(crate) proof: Vec<Challenge, 130>,
}

impl PokSignatureProof {
    /// Store the proof as a sequence of bytes
    /// Each point is compressed to big-endian format
    /// Needs (N + 2) * 32 + 48 * 2 + 96 space otherwise it will panic
    /// where N is the number of hidden messages
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        buffer[0..COMMITMENT_BYTES].copy_from_slice(&self.sigma_1.to_affine().to_compressed());
        let mut offset = COMMITMENT_BYTES;
        let mut end = offset + COMMITMENT_BYTES;
        buffer[offset..end].copy_from_slice(&self.sigma_2.to_affine().to_compressed());
        offset = end;
        end += 2 * COMMITMENT_BYTES;
        buffer[offset..end].copy_from_slice(&self.commitment.to_affine().to_compressed());
        offset = end;
        end += FIELD_BYTES;

        for i in 0..self.proof.len() {
            buffer[offset..end].copy_from_slice(&self.proof[i].to_bytes());
            offset = end;
            end += FIELD_BYTES;
        }
    }

    /// Convert a byte sequence into the blind signature context
    /// Expected size is (N + 2) * 32 + 48 * 2 bytes
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        let size = FIELD_BYTES * 3 + COMMITMENT_BYTES * 4;
        let buffer = bytes.as_ref();
        if buffer.len() < size {
            return None;
        }
        if buffer.len() % FIELD_BYTES != 0 {
            return None;
        }

        let hid_msg_cnt = (buffer.len() - COMMITMENT_BYTES * 4) / FIELD_BYTES;
        let mut offset = COMMITMENT_BYTES;
        let mut end = COMMITMENT_BYTES + COMMITMENT_BYTES;
        let sigma_1 = G1Affine::from_compressed(slicer!(buffer, 0, offset, COMMITMENT_BYTES))
            .map(G1Projective::from);
        let sigma_2 = G1Affine::from_compressed(slicer!(buffer, offset, end, COMMITMENT_BYTES))
            .map(G1Projective::from);
        offset = end;
        end += 2 * COMMITMENT_BYTES;
        let commitment =
            G2Affine::from_compressed(slicer!(buffer, offset, end, 2 * COMMITMENT_BYTES))
                .map(G2Projective::from);

        if sigma_1.is_none().unwrap_u8() == 1
            || sigma_2.is_none().unwrap_u8() == 1
            || commitment.is_none().unwrap_u8() == 1
        {
            return None;
        }

        offset = end;
        end += FIELD_BYTES;

        let mut proof = Vec::<Challenge, 130>::new();
        for _ in 0..hid_msg_cnt {
            let c = Challenge::from_bytes(slicer!(buffer, offset, end, FIELD_BYTES));
            offset = end;
            end = offset + FIELD_BYTES;
            if c.is_none().unwrap_u8() == 1 {
                return None;
            }

            proof.push(c.unwrap()).expect(ALLOC_MSG);
        }
        Some(Self {
            sigma_1: sigma_1.unwrap(),
            sigma_2: sigma_2.unwrap(),
            commitment: commitment.unwrap(),
            proof,
        })
    }

    /// Convert the committed values to bytes for the fiat-shamir challenge
    pub fn add_challenge_contribution(
        &self,
        public_key: &PublicKey,
        rvl_msgs: &[(usize, Message)],
        challenge: Challenge,
        hasher: &mut impl Update,
    ) {
        hasher.update(self.sigma_1.to_affine().to_uncompressed());
        hasher.update(self.sigma_2.to_affine().to_uncompressed());
        hasher.update(self.commitment.to_affine().to_uncompressed());

        let mut points = Vec::<G2Projective, 130>::new();

        points.push(G2Projective::generator()).expect(ALLOC_MSG);
        points.push(public_key.w).expect(ALLOC_MSG);

        let mut known = HashSet::new();
        for (idx, _) in rvl_msgs {
            known.insert(*idx);
        }

        for i in 0..public_key.y.len() {
            if known.contains(&i) {
                continue;
            }
            points.push(public_key.y[i]).expect(ALLOC_MSG);
        }
        points.push(self.commitment).expect(ALLOC_MSG);

        let mut scalars: Vec<Scalar, 130> = self.proof.iter().map(|p| p.0).collect();
        scalars.push(-challenge.0).expect(ALLOC_MSG);
        let commitment = G2Projective::sum_of_products_in_place(points.as_ref(), scalars.as_mut());
        hasher.update(commitment.to_affine().to_bytes());
    }

    /// Validate the proof, only checks the signature proof
    /// the selective disclosure proof is checked by verifying
    /// self.challenge == computed_challenge
    pub fn verify(&self, rvl_msgs: &[(usize, Message)], public_key: &PublicKey) -> bool {
        // check the signature proof
        if self
            .sigma_1
            .is_identity()
            .bitor(self.sigma_2.is_identity())
            .unwrap_u8()
            == 1
        {
            return false;
        }

        if public_key.y.len() < rvl_msgs.len() {
            return false;
        }

        let mut points = Vec::<G2Projective, 130>::new();
        let mut scalars = Vec::<Scalar, 130>::new();

        for (idx, msg) in rvl_msgs {
            if *idx > public_key.y.len() {
                return false;
            }
            points.push(public_key.y[*idx]).expect(ALLOC_MSG);
            scalars.push(msg.0).expect(ALLOC_MSG);
        }
        points.push(public_key.x).expect(ALLOC_MSG);
        scalars.push(Scalar::one()).expect(ALLOC_MSG);
        points.push(self.commitment).expect(ALLOC_MSG);
        scalars.push(Scalar::one()).expect(ALLOC_MSG);

        let j = G2Projective::sum_of_products_in_place(points.as_ref(), scalars.as_mut());

        multi_miller_loop(&[
            (&self.sigma_1.to_affine(), &G2Prepared::from(j.to_affine())),
            (
                &self.sigma_2.to_affine(),
                &G2Prepared::from(-G2Affine::generator()),
            ),
        ])
        .final_exponentiation()
        .is_identity()
        .unwrap_u8()
            == 1
    }
}
