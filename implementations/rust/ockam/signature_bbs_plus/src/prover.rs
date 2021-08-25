use crate::{BlindSignatureContext, MessageGenerators, PokSignature, Signature};
use blake2::VarBlake2b;
use bls12_381_plus::{G1Affine, G1Projective, Scalar};
use digest::{Update, VariableOutput};
use group::Curve;
#[cfg(feature = "unsafe_random")]
use rand_core::RngCore;
#[cfg(not(feature = "unsafe_random"))]
use rand_core::{CryptoRng, RngCore};
use signature_core::{error::Error, lib::*};

/// A Prover is whomever receives signatures or uses them to generate proofs.
/// Provided are methods for 2PC where some are only known to the prover and a blind signature
/// is created, unblinding signatures, verifying signatures, and creating signature proofs of knowledge
/// with selective disclosure proofs
pub struct Prover;

impl Prover {
    /// Create the structures need to send to an issuer to complete a blinded signature
    /// `messages` is an index to message map where the index corresponds to the index in `generators`
    pub fn new_blind_signature_context(
        messages: &[(usize, Message)],
        generators: &MessageGenerators,
        nonce: Nonce,
        #[cfg(not(feature = "unsafe_random"))] mut rng: impl RngCore + CryptoRng,
        #[cfg(feature = "unsafe_random")] mut rng: impl RngCore,
    ) -> Result<(BlindSignatureContext, SignatureBlinding), Error> {
        const BYTES: usize = 48;
        // Very uncommon to blind more than 1 or 2, so 16 should be plenty
        let mut points = Vec::<G1Projective, 16>::new();
        let mut secrets = Vec::<Scalar, 16>::new();
        let mut committing = ProofCommittedBuilder::<G1Projective, G1Affine, 16, 16>::new(
            G1Projective::sum_of_products_in_place,
        );

        for (i, m) in messages {
            if *i > generators.len() {
                return Err(Error::new(*i as u32, "invalid index"));
            }
            secrets.push(m.0).map_err(|_| {
                Error::new(
                    1,
                    "allocate more space: Prover::new_blind_signature_context",
                )
            })?;
            points.push(generators.get(*i)).map_err(|_| {
                Error::new(
                    2,
                    "allocate more space: Prover::new_blind_signature_context",
                )
            })?;
            committing.commit_random(generators.get(*i), &mut rng);
        }

        let blinding = SignatureBlinding::random(&mut rng);
        secrets.push(blinding.0).map_err(|_| {
            Error::new(
                3,
                "allocate more space: Prover::new_blind_signature_context",
            )
        })?;
        points.push(generators.h0).map_err(|_| {
            Error::new(
                4,
                "allocate more space: Prover::new_blind_signature_context",
            )
        })?;
        committing.commit_random(generators.h0, &mut rng);

        let mut hasher = VarBlake2b::new(BYTES).unwrap();
        let commitment = G1Projective::sum_of_products_in_place(points.as_ref(), secrets.as_mut());

        committing.add_challenge_contribution(&mut hasher);
        hasher.update(&commitment.to_affine().to_uncompressed());
        hasher.update(&nonce.to_bytes());
        let mut res = [0u8; BYTES];
        hasher.finalize_variable(|out| {
            res.copy_from_slice(out);
        });
        let challenge = Scalar::from_okm(&res);
        let proofs: Vec<Challenge, 16> = committing
            .generate_proof(challenge, secrets.as_ref())?
            .iter()
            .map(|s| Challenge(*s))
            .collect();
        Ok((
            BlindSignatureContext {
                commitment: Commitment(commitment),
                challenge: Challenge(challenge),
                proofs,
            },
            blinding,
        ))
    }

    /// Create a new signature proof of knowledge and selective disclosure proof
    /// from a verifier's request
    #[cfg(not(feature = "unsafe_random"))]
    pub fn commit_signature_pok(
        signature: Signature,
        generators: &MessageGenerators,
        messages: &[ProofMessage],
        rng: impl RngCore + CryptoRng,
    ) -> Result<PokSignature, Error> {
        PokSignature::init(signature, generators, messages, rng)
    }

    /// Create a new signature proof of knowledge and selective disclosure proof
    /// from a verifier's request
    #[cfg(feature = "unsafe_random")]
    pub fn commit_signature_pok(
        signature: Signature,
        generators: &MessageGenerators,
        messages: &[ProofMessage],
        rng: impl RngCore,
    ) -> Result<PokSignature, Error> {
        PokSignature::init(signature, generators, messages, rng)
    }
}

#[test]
fn blind_signature_context_test() {
    use crate::{util::MockRng, Issuer};
    use rand_core::*;

    let seed = [1u8; 16];
    let mut rng = MockRng::from_seed(seed);

    let (pk, sk) = Issuer::new_keys(&mut rng).unwrap();
    let generators = MessageGenerators::from_public_key(pk, 4);
    let nonce = Nonce::random(&mut rng);
    // let secret_id = Message::random(&mut rng);

    // try with zero, just means a blinded signature but issuer knows all messages
    let mut blind_messages = [];

    let res =
        Prover::new_blind_signature_context(&mut blind_messages[..], &generators, nonce, &mut rng);
    assert!(res.is_ok());

    let (ctx, blinding) = res.unwrap();

    let mut messages = [
        (0, Message::hash(b"firstname")),
        (1, Message::hash(b"lastname")),
        (2, Message::hash(b"age")),
        (3, Message::hash(b"allowed")),
    ];
    let res = Issuer::blind_sign(&ctx, &sk, &generators, &mut messages[..], nonce);
    assert!(res.is_ok());
    let blind_signature = res.unwrap();
    let signature = blind_signature.to_unblinded(blinding);

    let msgs = [messages[0].1, messages[1].1, messages[2].1, messages[3].1];

    let res = signature.verify(&pk, &generators, msgs.as_ref());
    assert_eq!(res.unwrap_u8(), 1);
}
