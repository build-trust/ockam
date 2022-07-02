#![allow(unused_variables)]
use signature_bbs_plus::{Issuer, MessageGenerators, Prover};
use signature_core::{error::Error, lib::*};

type ExampleResult = Result<(), Error>;

fn ex_new_keys() -> ExampleResult {
    let (public_key, secret_key) = Issuer::new_keys(&mut rand::thread_rng())?;
    Ok(())
}

fn ex_generator() -> ExampleResult {
    let (public_key, secret_key) = Issuer::new_keys(&mut rand::thread_rng())?;
    let num_messages = 4;
    let generators = MessageGenerators::from_public_key(public_key, num_messages);
    let messages = [
        Message::hash(b"message 1"),
        Message::hash(b"message 2"),
        Message::hash(b"message 3"),
        Message::hash(b"message 4"),
    ];

    let signature = Issuer::sign(&secret_key, &generators, &messages)?;
    Ok(())
}

fn ex_blinding() -> ExampleResult {
    let mut rng = rand::thread_rng();
    let (public_key, secret_key) = Issuer::new_keys(&mut rng)?;
    let num_messages = 4;
    let generators = MessageGenerators::from_public_key(public_key, num_messages);
    let nonce = Nonce::random(&mut rng);

    let (context, blinding) =
        Prover::new_blind_signature_context(&[][..], &generators, nonce, &mut rng)?;
    let messages = [
        (0, Message::hash(b"firstname")),
        (1, Message::hash(b"lastname")),
        (2, Message::hash(b"age")),
        (3, Message::hash(b"allowed")),
    ];

    let blind_signature =
        Issuer::blind_sign(&context, &secret_key, &generators, &messages[..], nonce)?;

    Ok(())
}

fn ex_unblind_and_verify() -> ExampleResult {
    let mut rng = rand::thread_rng();
    let (public_key, secret_key) = Issuer::new_keys(&mut rng)?;
    let num_messages = 4;
    let generators = MessageGenerators::from_public_key(public_key, num_messages);
    let nonce = Nonce::random(&mut rng);

    let (context, blinding) =
        Prover::new_blind_signature_context(&[][..], &generators, nonce, &mut rng)?;
    let messages = [
        (0, Message::hash(b"firstname")),
        (1, Message::hash(b"lastname")),
        (2, Message::hash(b"age")),
        (3, Message::hash(b"allowed")),
    ];

    let blind_signature =
        Issuer::blind_sign(&context, &secret_key, &generators, &messages[..], nonce)?;

    let signature = blind_signature.to_unblinded(blinding);

    // Remove index
    let messages = [messages[0].1, messages[1].1, messages[2].1, messages[3].1];

    let res = signature.verify(&public_key, &generators, messages.as_ref());
    assert_eq!(res.unwrap_u8(), 1);

    Ok(())
}

fn main() -> Result<(), Error> {
    ex_new_keys()?;
    ex_generator()?;
    ex_blinding()?;
    ex_unblind_and_verify()
}
