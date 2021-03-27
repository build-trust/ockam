---
title: Credentials
order: 3
---

# Glossary

- **[Attribute](https://docs.rs/ockam/0.3.0/ockam/enum.CredentialAttribute.html)**: Information such as a property or a tag.
- **Claim**: Attributes that have been digitally signed for inclusion in a Credential.
- **[Credential](https://docs.rs/ockam/0.3.0/ockam/struct.Credential.html)**: A set of Claims and their digital signature. Provided by an Issuer to Holders.
- **[Holder](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialHolder.html)**: An entity which posesses a Credential.
- **[Verifier](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialVerifier.htmll)**: An entity which can prove a Credential is trusted by an Issuer.
- **[Schema](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialSchema.html)**: The set of Claims and Attributes required by a Verifier.
- **[Issuer](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialIssuer.html)**: A trusted entity which provides Credentials to a Holder.
- **[Credential Offer](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialOffer.html)**: A message sent from the Issuer to a Holder containing information about obtaining a credential.
- **[Credential Request](https://docs.rs/ockam/0.3.0/ockam/struct.CredentialRequest.html)**: A message sent from the Holder to the Issuer, in response to a Credential Offer. This message indicates that the Holder wants the Issuer to produce a Credential.

# Ockam Credentials

This example shows how to use the Ockam Credentials API exchange trusted Credentials between Holders, Issuers, and Verifiers.

To obtain a Credential, the following exchange protocol takes place:

1. The Holder receives a references to the Issuer service. This message contains meta-information about the Issuer.
2. The Holder sends a "New Credential" message to the Issuer.
3. The Issuer replies to the Holder with a Credential Offer.
4. The Holder uses the Credential Offer to build and send a Credential Request.
5. The Issuer replies to the Holder with the actual Credential.

## Credential Example Setup

Clone the ockam repository and change into the credential example directory:

```bash
git clone git@github.com:ockam-network/ockam.git
cd implementations/rust/examples/credentials
cargo build
```

The credential example is split into three programs: `issuer`, `holder`, and `verifier`

### Running the Issuer

Run the `issuer` program:

```bash
target/debug/issuer
```

You will see output showing that it is listening on a local socket:

```
Listening on "issuer.socket"
```

### Running the Holder

Run the `holder` program:

```bash
target/debug/holder
```

Iin the `issuer` program console, you will see information about credentials offered to the holder:

```bash
Listening on "issuer.socket"
Client closed connection
Unhandled message: Presentation([CredentialPresentation { presentation_id: [75, 220, 177, 200, 151, 101, 200, 31, 39, 5, 98, 33, 139, 18, 240, 146, 247, 65, 178, 102, 36, 188, 129, 255, 241, 249, 245, 230, 109, 179, 171, 122], revealed_attributes: [Numeric(1)], proof: PoKOfSignatureProof { a_prime: G1 { x: Fq(0x14d63fc0ea59c2339d16e0badc05b5c57188e967c00ad07566b02d30a4d0b4ea989697d14ccb9d7b1ec9ba5d2bfc6cdb), y: Fq(0x10ec3a7fe5433f13654c31dd962e86c73dfc8f82f8fbeae38d15293641623bdc5484f210e48032cfb106a97766671128), z: Fq(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }, a_bar: G1 { x: Fq(0x10f295c8092ad84a38d382dacf73415806d952e1a763f95c9424f930d66fa4d6cc70d7873f18a80251fe390e8af12651), y: Fq(0x12aba09fd510325f762e396a95f5fde275aa520dff918098758620f0ec5c1222a8fa5636c79d16d2f24fca58d0399746), z: Fq(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }, d: G1 { x: Fq(0x065a760b1ad2ca6543576eb1b7ff3bb2e4e24a66802ddd3d0ba21b5315a7d351967c6df17c7d028dcc64c9a16b08edaa), y: Fq(0x0c218064762dddfb2b2efaeca8e82c3b92410ce241f7dcab174e1c1f4dcb25c00d0806735afbf297da48700b4553480b), z: Fq(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }, proof_vc_1: ProofG1 { commitment: G1 { x: Fq(0x16e14ea5826bcac7f08e77097fb3a4b12d4a0e0f20c01e885d596174a45f5ce0bbb338feab680119485bb5f66bf482de), y: Fq(0x0e31586065ab874020028d7f57a7710b66db88d2d9fe7d6773f5e8ce481145166d3fe1d8c760fa8ba15e013ebf5cf3b6), z: Fq(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }, responses: [Fr(0x2b2639f4618587502c075fdcdcf6bb87dac2785a3952e422aab22d7045f697b8), Fr(0x30e70b996f3146e7809b534c21c46e9e8fd6c8bd9cc1ca0c3603669a59826d7e)] }, proof_vc_2: ProofG1 { commitment: G1 { x: Fq(0x08f9f6dcc3d9b001b28615df81b6cb95da016b9cfc28a581c3058656370bca394da1a3e669a982d63c0892938ad01e80), y: Fq(0x09836567a4c07104e85d21439b87423eded60e8ee8c675541f8fd94f3991507904bf2ec11864e08d3b43d90ec9640aff), z: Fq(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }, responses: [Fr(0x13d9162a4b0e0c39e10d3063d1775791b82e0d84322937f99d8e48a7126fa5c9), Fr(0x234fba6cb6a99cf62ec71434bdc5cdfe167158c9fce211f7fb867888a4d31d43), Fr(0x1509ba5c07834e3febd69c12b3e31f464e5f99fe88d8c4f763915be6e065a401)] } } }])
Client closed connection
```

### Verifier

WIP

### Credential API Deep Dive

We create a new Issuer with the given (or generated) secret key. We then create some other necessary objects,
such as our proof of posession and credential schema.

```rust
let args = Args::from_args();

// Create a new issuer.
// CredentialIssuer has a credential signature public key.
// CredentialIssuer creates a proof of possession
// so users can verify it.
// These should be posted such that verifiers and
// holders can check them
let issuer = if args.secret_key.is_none() {
    CredentialIssuer::new()
} else {
    let sk = <[u8; 32]>::try_from(
        hex::decode(args.secret_key.as_ref().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    CredentialIssuer::with_signing_key(sk)
};
let pk = issuer.get_public_key();
let pop = issuer.create_proof_of_possession();
let schema = example_schema();
```

We then enter a message processing loop which completes the credential flow:

```rust
match m {
    CredentialMessage::CredentialConnection => {
        serde_bare::to_writer(
            &mut stream,
            &CredentialMessage::CredentialIssuer {
                public_key: pk,
                proof: pop,
            },
        )
        .unwrap();
        stream.flush().unwrap();
    }
    CredentialMessage::NewCredential => {
        // CredentialIssuer offers holder a credential
        let offer = issuer.create_offer(&schema);
        pending_offers.insert(offer.id);
        serde_bare::to_writer(&mut stream, &CredentialMessage::CredentialOffer(offer))
            .unwrap();
        stream.flush().unwrap();
    }
    CredentialMessage::CredentialRequest(request) => {
        if !pending_offers.contains(&request.offer_id) {
            eprintln!("Unexpected offer id: {:?}", request.offer_id);
            serde_bare::to_writer(
                &mut stream,
                &CredentialMessage::InvalidCredentialRequest,
            )
            .unwrap();
            stream.flush().unwrap();
            continue;
        }
        // CredentialIssuer processes the credential request
        // Issuer knows all of the attributes that were not blinded
        // by the holder
        let mut attributes = BTreeMap::new();
        attributes.insert(
            schema.attributes[1].label.clone(),
            CredentialAttribute::Numeric(1), // TRUE, the device has access
        );

        // Fragment 2 is a partial signature
        let credential_fragment2 = issuer
            .sign_credential_request(&request, &schema, &attributes, request.offer_id)
            .unwrap();
        serde_bare::to_writer(
            &mut stream,
            &CredentialMessage::CredentialResponse(credential_fragment2),
        )
        .unwrap();
        stream.flush().unwrap();
        pending_offers.remove(&request.offer_id);
    }
    _ => {
        eprintln!("Unhandled message: {:?}", m);
    }
}
```

TBD. See the [example source code.](https://github.com/ockam-network/ockam/tree/develop/implementations/rust/examples/credentials/src)

## Holder API

We first ask for a new credential, and receive back a credential offer:

```rust
// Ask for a new credential
serde_bare::to_writer(&mut stream, &CredentialMessage::NewCredential)
    .expect("Unable to ask for new credential from issuer");
stream.flush().unwrap();

let offer;
let reader = stream.try_clone().unwrap();
let msg = serde_bare::from_reader::<Stream, CredentialMessage>(reader)
    .expect("Unable to read message from issuer");

if let CredentialMessage::CredentialOffer(o) = msg {
    offer = o;
} else {
    eprintln!("Unexpected message returned from Issuer");
    return;
}
```

The holder accepts the credential offer. Accepting the offer yields a request to send back to
the issuer, as well as the first credential fragment. The first fragment is held until the issuer sends
the second credential fragment. The fragments are combined to produce the credential. Fragment 1 is a cryptographic commitment that hides the
the holder's unique id. The unique id is used to prove that multiple credentials were issued to the same holder:

```rust
let (request, credential_fragment1) = holder.accept_credential_offer(&offer, pk).unwrap();

// Send the request
serde_bare::to_writer(&mut stream, &CredentialMessage::CredentialRequest(request))
    .expect("Unable to send credential request");
stream.flush().unwrap();

let reader = stream.try_clone().unwrap();
let msg = serde_bare::from_reader::<Stream, CredentialMessage>(reader)
    .expect("Unable to read message from issuer");

let credential;
if let CredentialMessage::CredentialResponse(credential_fragment2) = msg {
    credential =
        holder.combine_credential_fragments(credential_fragment1, credential_fragment2);
} else {
    eprintln!("Unexpected message returned from Issuer");
    return;
}
```

The holder now has possession of credentials that can be provably trusted. We can
prove this to verifiers by using a presentation manifest:

```rust
let presentation_manifest = PresentationManifest {
    credential_schema: example_schema(), // only accept credentials that match this schema
    public_key: pk,                      // only accept credentials issued by this authority
    revealed: vec![1],                   // location is required to be revealed
};
...
let presentation = holder
.present_credentials(&[credential], &[presentation_manifest.clone()], request_id)
.unwrap();
serde_bare::to_writer(&mut stream, &CredentialMessage::Presentation(presentation))
.expect("Unable to send presentation");

```
