# Step 3 - Identity

[[Entity]] [[Identity API]] [[Profile]] [[Secure Channel]]

## Building Trust with Nodes and Workers

Ockam provides secure methods of establishing and verifying trust, built on a foundation of nodes and workers. 

Ockam defines security protocols used in the establishment of trust. A security protocol is a strictly typed and ordered message flow between parties.

Security protocols include:

- **Key Agreement Protocol**: Computes shared secrets between two parties without transmitting the secret.
- **Secure Channel Protocol**: Encrypts and decrypts data.
- **Signing and Verifying Protocol**: Signs and verifies data.
- **Credential Request Protocol**: Blind signature credential issuance.
- **Credential Verification Protocol**: Blind signature credential verification.

## Identity API

The Identity API offers small, simplified interface to the more complex security protocols. The Identity API provides the features of the underlying protocols, while handling implementation details.

The Identity API provides:

- Cryptographic key creation, rotation and retrieval
- Cryptographic proof creation and verification mechanism
- Secure Channel establishment
- Credential issuance and verification
- Change verification
- Contact management

## Entity

An Entity is a worker that implements the Identity API. Entities are a central concept within the security features of the Rust SDK. The interaction between multiple parties establishing trust is modeled by using Entities.

An Entity is created by calling the `Entity::create` function.

```rust
use ockam::Entity;
...
let alice = Entity::create(&ctx)?;
```

Entity implements the `Identity` trait, which defines the Identity API.

Entities contain their own state and secrets. Secrets are stored in a secure system called a Vault. The identity that an Entity represents is based on its cryptographic key pair stored in the Vault.

This identifier is called the Profile Identifier. A Profile is a specific identifier backed by a keypair. An Entity can have multiple Profiles, by having multiple keypairs in the Vault.

The ability for an Entity to have multiple Profiles enhances the privacy of an Entity. Two Profiles belonging to an Entity cannot be associated with one another, or back to the Entity. This allows a single real user to use multiple Profiles, each for a different identity scenario.

For example, a user may have a Manufacturer Identity for technical support, and an Advertiser Identity for third party integrations.

Like an Entity, a Profile also implements the Identity API. In many Ockam APIs, Entities and Profiles can be used interchangeably. 

## Profiles

An Entity has a default Profile which is created automatically. This Profile can be accessed by calling the `Entity::current_profile` function. A new Profile can be created by calling `Entity::create_profile`, and removed with `Entity::remove_profile`.

```rust
// Create an Entity
let mut alice = Entity::create(&ctx)?;

// Get the default profile
let alice_default_profile = alice.current_profile().unwrap();  

// Create a new profile
let alice_manufacturer = alice.create_profile()?;

// Delete a profile
alice.remove_profile(alice_manufacturer.identifier()?)?;
```
