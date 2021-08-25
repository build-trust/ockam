use bls12_381_plus::{G1Affine, G1Projective};
use group::Curve;
use heapless::Vec as HVec;
use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use signature_bbs_plus::{BlindSignatureContext, PokSignatureProof};

use signature_core::lib::{Challenge, Commitment};
big_array! { BigArray; }

/// External representation of a Commitment. G1Projective Serialization is interfering with BARE
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtCommitment {
    /// External representation of Commitment G1
    #[serde(with = "BigArray")]
    pub g1_bytes: [u8; 48],
}

fn bytes_to_g1p(bytes: &[u8; 48]) -> G1Projective {
    G1Projective::from(G1Affine::from_compressed(bytes).unwrap())
}

fn g1p_to_bytes(g1p: G1Projective) -> [u8; 48] {
    g1p.to_affine().to_compressed()
}

impl From<ExtCommitment> for Commitment {
    /// Convert ExtCommitment to Commitment
    fn from(ext: ExtCommitment) -> Self {
        Commitment(bytes_to_g1p(&ext.g1_bytes))
    }
}

impl ExtCommitment {
    /// Convert a Commitment into ExtCommitment
    pub fn from(commitment: Commitment) -> Self {
        ExtCommitment {
            g1_bytes: commitment.to_bytes(),
        }
    }
}

/// Temporary: Externalized form of BlindSignatureContext - working around Serialize issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtBlindSignatureContext {
    /// The blinded signature commitment
    pub commitment: ExtCommitment,
    /// The challenge hash for the Fiat-Shamir heuristic
    pub challenge: Challenge,
    /// The proofs for the hidden messages
    pub proofs: Vec<Challenge>,
}

impl From<ExtBlindSignatureContext> for BlindSignatureContext {
    fn from(ext: ExtBlindSignatureContext) -> Self {
        let mut proofs: HVec<Challenge, 16> = HVec::new();
        for proof in ext.proofs {
            proofs.push(proof).unwrap();
        }
        BlindSignatureContext {
            commitment: ext.commitment.into(),
            challenge: ext.challenge,
            proofs,
        }
    }
}

impl From<BlindSignatureContext> for ExtBlindSignatureContext {
    fn from(sig: BlindSignatureContext) -> Self {
        let proofs = sig.proofs.to_vec();
        ExtBlindSignatureContext {
            commitment: ExtCommitment::from(sig.commitment),
            challenge: sig.challenge,
            proofs,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Externalization of PoK
pub struct ExtPokSignatureProof {
    /// a_prime
    #[serde(with = "BigArray")]
    pub a_prime: [u8; 48],

    /// temp
    #[serde(with = "BigArray")]
    pub a_bar: [u8; 48],

    /// d
    #[serde(with = "BigArray")]
    pub d: [u8; 48],

    /// proofs1
    pub proofs1: [Challenge; 2],

    /// proofs 2
    pub proofs2: Vec<Challenge>,
}

impl From<ExtPokSignatureProof> for PokSignatureProof {
    fn from(ext: ExtPokSignatureProof) -> Self {
        let mut proofs2: HVec<Challenge, 130> = HVec::new();
        for proof in ext.proofs2 {
            proofs2.push(proof).unwrap();
        }

        PokSignatureProof {
            a_prime: bytes_to_g1p(&ext.a_prime),
            a_bar: bytes_to_g1p(&ext.a_bar),
            d: bytes_to_g1p(&ext.d),
            proofs1: ext.proofs1,
            proofs2,
        }
    }
}

impl From<PokSignatureProof> for ExtPokSignatureProof {
    fn from(pok: PokSignatureProof) -> Self {
        ExtPokSignatureProof {
            a_prime: g1p_to_bytes(pok.a_prime),
            a_bar: g1p_to_bytes(pok.a_bar),
            d: g1p_to_bytes(pok.d),
            proofs1: pok.proofs1,
            proofs2: pok.proofs2.to_vec(),
        }
    }
}
