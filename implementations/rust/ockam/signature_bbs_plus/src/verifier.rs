use crate::{MessageGenerators, PokSignatureProof};
use blake2::VarBlake2b;
use bls12_381_plus::Scalar;
use digest::{Update, VariableOutput};
use rand_core::{CryptoRng, RngCore};
use signature_bls::PublicKey;
use signature_core::{constants::*, lib::*};

/// This struct represents an Verifier of signatures.
/// Provided are methods for generating a context to ask for revealed messages
/// and the prover keep all others hidden.
pub struct Verifier;

impl Verifier {
    /// Create a nonce used for the proof request context
    pub fn generate_proof_nonce(rng: impl RngCore + CryptoRng) -> Nonce {
        Nonce::random(rng)
    }

    /// Check a signature proof of knowledge and selective disclosure proof
    pub fn verify_signature_pok(
        revealed_msgs: &[(usize, Message)],
        public_key: PublicKey,
        proof: PokSignatureProof,
        generators: &MessageGenerators,
        nonce: Nonce,
        challenge: Challenge,
    ) -> bool {
        let mut res = [0u8; COMMITMENT_BYTES];
        let mut hasher = VarBlake2b::new(COMMITMENT_BYTES).unwrap();
        proof.add_challenge_contribution(generators, revealed_msgs, challenge, &mut hasher);
        hasher.update(&nonce.to_bytes()[..]);
        hasher.finalize_variable(|out| {
            res.copy_from_slice(out);
        });
        let v_challenge = Scalar::from_okm(&res);

        proof.verify(public_key) && challenge.0 == v_challenge
    }
}

#[test]
fn pok_sig_proof_works() {
    use crate::{util::MockRng, Issuer, PokSignature};
    use rand_core::*;

    let seed = [1u8; 16];
    let mut rng = MockRng::from_seed(seed);

    let (pk, sk) = Issuer::new_keys(&mut rng).unwrap();
    let generators = MessageGenerators::from_public_key(pk, 4);
    let messages = [
        Message::random(&mut rng),
        Message::random(&mut rng),
        Message::random(&mut rng),
        Message::random(&mut rng),
    ];

    let res = Issuer::sign(&sk, &generators, &messages);
    assert!(res.is_ok());

    let signature = res.unwrap();

    let proof_messages = [
        ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(messages[0])),
        ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(messages[1])),
        ProofMessage::Revealed(messages[2]),
        ProofMessage::Revealed(messages[3]),
    ];

    let res = PokSignature::init(signature, &generators, &proof_messages, &mut rng);
    assert!(res.is_ok());

    let mut tv = [0u8; 48];
    let mut pok_sig = res.unwrap();
    let nonce = Verifier::generate_proof_nonce(&mut rng);
    let mut hasher = VarBlake2b::new(COMMITMENT_BYTES).unwrap();
    pok_sig.add_proof_contribution(&mut hasher);
    hasher.update(&nonce.to_bytes()[..]);
    hasher.finalize_variable(|out| {
        tv.copy_from_slice(out);
    });
    let challenge = Challenge::from_okm(&tv);
    let res = pok_sig.generate_proof(challenge);
    assert!(res.is_ok());

    let proof = res.unwrap();
    assert!(proof.verify(pk));

    let mut hasher = VarBlake2b::new(COMMITMENT_BYTES).unwrap();
    proof.add_challenge_contribution(
        &generators,
        &[(2, messages[2]), (3, messages[3])][..],
        challenge,
        &mut hasher,
    );
    hasher.update(&nonce.to_bytes()[..]);
    hasher.finalize_variable(|out| {
        tv.copy_from_slice(out);
    });
    let challenge2 = Challenge::from_okm(&tv);
    assert_eq!(challenge, challenge2);

    assert!(Verifier::verify_signature_pok(
        &[(2, messages[2]), (3, messages[3])][..],
        pk,
        proof,
        &generators,
        nonce,
        challenge
    ));
}
