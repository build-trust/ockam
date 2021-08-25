use crate::{MessageGenerators, PokSignatureProof, Signature};
use bls12_381_plus::{G1Affine, G1Projective, Scalar};
use core::ops::Neg;
use digest::Update;
use ff::Field;
use group::Curve;
#[cfg(feature = "unsafe_random")]
use rand_core::RngCore;
#[cfg(not(feature = "unsafe_random"))]
use rand_core::{CryptoRng, RngCore};
use signature_core::{error::Error, lib::*};

/// Proof of Knowledge of a Signature that is used by the prover
/// to construct `PoKOfSignatureProof`.
pub struct PokSignature {
    /// A' in section 4.5
    a_prime: G1Projective,
    /// \overline{A} in section 4.5
    a_bar: G1Projective,
    /// d in section 4.5
    d: G1Projective,
    /// For proving relation a_bar / d == a_prime^{-e} * h_0^r2
    proof1: ProofCommittedBuilder<G1Projective, G1Affine, 2, 2>,
    /// The messages
    secrets1: [Scalar; 2],
    /// For proving relation g1 * h1^m1 * h2^m2.... for all disclosed messages m_i == d^r3 * h_0^{-s_prime} * h1^-m1 * h2^-m2.... for all undisclosed messages m_i
    /// 130 because 128 messages + 2 extra for blinding
    proof2: ProofCommittedBuilder<G1Projective, G1Affine, 130, 130>,
    /// The blinding factors
    secrets2: Vec<Scalar, 130>,
}

impl PokSignature {
    /// Creates the initial proof data before a Fiat-Shamir calculation
    pub fn init(
        signature: Signature,
        generators: &MessageGenerators,
        messages: &[ProofMessage],
        #[cfg(not(feature = "unsafe_random"))] mut rng: impl RngCore + CryptoRng,
        #[cfg(feature = "unsafe_random")] mut rng: impl RngCore,
    ) -> Result<Self, Error> {
        if messages.len() != generators.len() {
            return Err(Error::new(1, "mismatched messages with and generators"));
        }

        let r1 = Scalar::random(&mut rng);
        let r2 = Scalar::random(&mut rng);
        let r3 = r1.invert().unwrap();

        let m = messages
            .iter()
            .map(|m| m.get_message())
            .collect::<Vec<Message, 128>>();

        let b = Signature::compute_b(signature.s, m.as_ref(), generators);

        let a_prime = signature.a * r1;
        let a_bar = b * r1 - a_prime * signature.e;

        // d = b * r1 + h0 * r2
        let d = G1Projective::sum_of_products_in_place(&[b, generators.h0], [r1, r2].as_mut());

        // s' = s - r2 r3
        let s_prime = signature.s + r2 * r3;

        // For proving relation a_bar / d == a_prime^{-e} * h_0^r2
        let mut proof1 = ProofCommittedBuilder::<G1Projective, G1Affine, 2, 2>::new(
            G1Projective::sum_of_products_in_place,
        );
        // For a_prime * -e
        proof1.commit_random(a_prime, &mut rng);
        // For h0 * r2
        proof1.commit_random(generators.h0, &mut rng);
        let secrets1 = [signature.e, r2];

        let mut proof2 = ProofCommittedBuilder::<G1Projective, G1Affine, 130, 130>::new(
            G1Projective::sum_of_products_in_place,
        );
        let mut secrets2 = Vec::new();
        // for d * -r3
        proof2.commit_random(d.neg(), &mut rng);
        secrets2.push(r3).expect("allocate more space");
        // for h0 * s_prime
        proof2.commit_random(generators.h0, &mut rng);
        secrets2.push(s_prime).expect("allocate more space");

        #[allow(clippy::needless_range_loop)]
        for i in 0..generators.len() {
            match messages[i] {
                ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(m)) => {
                    proof2.commit_random(generators.get(i), &mut rng);
                    secrets2.push(m.0).expect("allocate more space");
                }
                ProofMessage::Hidden(HiddenMessage::ExternalBlinding(m, e)) => {
                    proof2.commit(generators.get(i), e.0);
                    secrets2.push(m.0).expect("allocate more space");
                }
                _ => {}
            }
        }

        Ok(Self {
            a_prime,
            a_bar,
            d,
            proof1,
            secrets1,
            proof2,
            secrets2,
        })
    }

    /// Convert the committed values to bytes for the fiat-shamir challenge
    pub fn add_proof_contribution(&mut self, hasher: &mut impl Update) {
        hasher.update(self.a_prime.to_affine().to_uncompressed());
        hasher.update(self.a_bar.to_affine().to_uncompressed());
        hasher.update(self.d.to_affine().to_uncompressed());
        self.proof1.add_challenge_contribution(hasher);
        self.proof2.add_challenge_contribution(hasher);
    }

    /// Generate the Schnorr challenges for the selective disclosure proofs
    pub fn generate_proof(self, challenge: Challenge) -> Result<PokSignatureProof, Error> {
        let proof1 = self
            .proof1
            .generate_proof(challenge.0, self.secrets1.as_ref())?;
        let proofs1 = [Challenge(proof1[0]), Challenge(proof1[1])];
        let proofs2: Vec<Challenge, 130> = self
            .proof2
            .generate_proof(challenge.0, self.secrets2.as_ref())?
            .iter()
            .map(|s| Challenge(*s))
            .collect();
        Ok(PokSignatureProof {
            a_prime: self.a_prime,
            a_bar: self.a_bar,
            d: self.d,
            proofs1,
            proofs2,
        })
    }
}
