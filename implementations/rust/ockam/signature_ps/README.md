# signature_ps

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

In order to support a variety of proving protocols, this crate implements the PS signature scheme which can be used to generate zero-knowledge proofs about signed attributes and the signatures themselves.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
signature_ps = "0.34.0"
```

## Crate Features

```
[dependencies]
signature_ps = { version = "0.34.0" , default-features = false }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`signature_ps` default features, Cargo will build `signature_ps` with the std
feature enabled whether or not your direct dependency on `signature_ps`
has `default-features = false`.

## API

### Generating Keys

The PS scheme allows the Signer and Holder to be two separate parties. This is often the case, particularly in the
case of [verifiable credentials](https://www.w3.org/TR/vc-data-model/).

To generate a new key pair for signing, call the `Issuer::new_keys` API. A Short Group Signature allows a set of messages
to be signed with a single key. PS can sign any number of messages at the expense of a bigger public key. This implementation
uses curve BLS12-381 and Blake2b-512 as a hash.

```rust
let (public_key, secret_key) = Issuer::new_keys(&mut rand::thread_rng())?;
```

### Message Generators

Message Generators are per-message cryptographic information input into the BBS+ algorithm. They are derived from the
public key, and the number of messages the key will be used to sign.

### Signing

To sign messages, call the `Issuer::sign` API.

```rust
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
```

### Blinding Signatures

To create blind signatures, we first need to establish a blind signature context. This is done with the `Prover::new_blind_signature_context`
API. This function takes an optional slice of pre-committed messages. In this example, an empty slice is used, indicating
no pre-committed messages. The generators, a random nonce, and the RNG are also used.

With the context and secret key, the blind signature is created by calling `Issuer::blind_sign`.

```rust
let nonce = Nonce::random(&mut rng);

let (context, blinding) =
    Prover::new_blind_signature_context(&mut [][..], &generators, nonce, &mut rng)?;
let mut messages = [
    (0, Message::hash(b"firstname")),
    (1, Message::hash(b"lastname")),
    (2, Message::hash(b"age")),
    (3, Message::hash(b"allowed")),
];

let blind_signature =
    Issuer::blind_sign(&context, &secret_key, &mut messages[..], nonce)?;
```

### Unblinding Signatures

Unblinding the signature uses the `blinding` information provided by the blinding signature context. The function `to_unblinded`
takes the `blinding` and returns a `Signature`.

```rust
let signature = blind_signature.to_unblinded(blinding);
```

### Verification

Once the signature has been unblinded, it can be used to verify the messages, using the public key. This is done by calling
the `Signature::verify` function. Calling `Choice::unwrap_u8` on the result of `verify` returns 1 when verification succeeds.

```rust
let signature = blind_signature.to_unblinded(blinding);

let messages = [
    Message::hash(b"message 1"),
    Message::hash(b"message 2"),
    Message::hash(b"message 3"),
    Message::hash(b"message 4"),
];
let res = signature.verify(&public_key, &generators, messages.as_ref());
assert_eq!(res.unwrap_u8(), 1);
```

## Full Example - Blinding, Unblinding, Verifying

```rust
use short_group_signatures_core::{error::Error, lib::*};
use signature_ps::{Issuer, MessageGenerators, Prover};

fn main() -> Result<(), Error> {
    let mut rng = rand::thread_rng();
    let (public_key, secret_key) = Issuer::new_keys(&mut rng)?;
    let num_messages = 4;
    let generators = MessageGenerators::from_secret_key(num_messages, &secret_key);
    let nonce = Nonce::random(&mut rng);

    let (context, blinding) =
        Prover::new_blind_signature_context(&mut [][..], &generators, nonce, &mut rng)?;
    let mut messages = [
        (0, Message::hash(b"firstname")),
        (1, Message::hash(b"lastname")),
        (2, Message::hash(b"age")),
        (3, Message::hash(b"allowed")),
    ];

    let blind_signature =
        Issuer::blind_sign(&context, &secret_key, &mut messages[..], nonce)?;

    let signature = blind_signature.to_unblinded(blinding);

    // Remove index
    let messages = [messages[0].1, messages[1].1, messages[2].1, messages[3].1];

    let res = signature.verify(&public_key, messages.as_ref());
    assert_eq!(res.unwrap_u8(), 1);
    Ok(())
}

```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam
[crate-image]: https://img.shields.io/crates/v/signature_ps.svg
[crate-link]: https://crates.io/crates/signature_ps

[docs-image]: https://docs.rs/signature_ps/badge.svg
[docs-link]: https://docs.rs/signature_ps

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
