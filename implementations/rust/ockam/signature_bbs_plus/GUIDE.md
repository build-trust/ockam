# Creating privacy-preserving signatures in Rust with BBS+

BBS+ is a short-group digital-signature scheme that allows selective disclosure
of signed messages.

* A `signer` can sign a collection of messages __with a single signature__ to
attest to the information contained in those messages.

* A `holder` collects this signature from the `signer` and then presents a fast
and small zero-knowledge proof-of-knowledge of the signature to a `verifier`.

* A `verifier` knows the signer's public key and trusts information that is
attested by the `signer`.

* With BBS+, the `holder` can then selectively reveal a subset of signature
messages covered by one signature.

* In typical digital signature schemes a `holder` has to reveal the entire signed
data (all messages) to a `verifier` .. this typically includes one or more identifiers
of the subject of an attestation and other information that a verifier doesn't need
to know.

* Since BBS+ allows for selective disclosure, it allows us to design anonymous,
privacy preserving attestations and credentials.

Let's step thorough how we can create such credentials with
the `signature_bbs_plus` crate:

### Generating Keys

To generate a new key pair for signing, call the `Issuer::new_keys` API. A
Short Group Signature allows a set of messages to be signed with a single key.
BBS+ can sign any number of messages at the expense of a bigger public key.
This implementation uses curve BLS12-381 and Blake2b-512 as a hash.

```rust
let (public_key, secret_key) = Issuer::new_keys(&mut rand::thread_rng())?;
```

### Message Generators

Message Generators are per-message cryptographic information input into the
BBS+ algorithm. They are derived from the public key, and the number of
messages the key will be used to sign.

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

To create blind signatures, we first need to establish a blind signature context.
This is done with the `Prover::new_blind_signature_context` API. This function
takes an optional slice of pre-committed messages. In this example, an empty
slice is used, indicating no pre-committed messages. The generators, a random
nonce, and the RNG are also used.

With the context and secret key, the blind signature is created by
calling `Issuer::blind_sign`.

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
    Issuer::blind_sign(&context, &secret_key, &generators, &mut messages[..], nonce)?;
```

### Unblinding Signatures

Unblinding the signature uses the `blinding` information provided by the
blinding signature context. The function `to_unblinded` takes the `blinding`
and returns a `Signature`.

```rust
let signature = blind_signature.to_unblinded(blinding);
```

### Verification

Once the signature has been unblinded, it can be used to verify the messages,
using the public key. This is done by calling the `Signature::verify` function.
Calling `Choice::unwrap_u8` on the result of `verify` returns 1 when verification succeeds.

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
use signature_bbs_plus::{Issuer, MessageGenerators, Prover};

fn main() -> Result<(), Error> {
    let mut rng = rand::thread_rng();
    let (public_key, secret_key) = Issuer::new_keys(&mut rng)?;
    let num_messages = 4;
    let generators = MessageGenerators::from_public_key(public_key, num_messages);
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
        Issuer::blind_sign(&context, &secret_key, &generators, &mut messages[..], nonce)?;

    let signature = blind_signature.to_unblinded(blinding);

    // Remove index
    let messages = [messages[0].1, messages[1].1, messages[2].1, messages[3].1];

    let res = signature.verify(&public_key, &generators, messages.as_ref());
    assert_eq!(res.unwrap_u8(), 1);
    Ok(())
}

```
