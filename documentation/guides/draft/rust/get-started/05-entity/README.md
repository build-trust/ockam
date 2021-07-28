```yaml
title: Entities
```

# Entities

## Vaults and Entities

Ockam protocols like secure channels, key lifecycle, credential
exchange, and device enrollment depend on a variety of standard
cryptographic primitives or building blocks. Depending on the environment,
these building blocks may be provided by a software implementation or a
cryptographically capable hardware component.

To support a variety of security hardware, there is loose coupling between
Ockam security protocols' building blocks and the underlying specific hardware
implementation. This is achieved using an abstract notion called Vault. A
software vault worker implementation is available to Ockam nodes. Over time,
and with help from the Ockam open source community, we plan to add vaults for
several TEEs, TPMs, HSMs, and Secure Enclaves.

A vault is used by a top level worker called an entity. Entities offer a small,
simplified interface to complex security protocols. They provide the features
of the underlying protocols, while handling implementation details. The
interaction between multiple parties establishing trust is modeled using Entities.

Entities provide:
- Cryptographic key creation, rotation and retrieval
- Cryptographic proof creation and verification mechanism
- Secure Channel establishment
- Credential issuance and verification
- Change verification
- Contact management

Entities and Vaults are built by calling their `create` functions. Both create
require a reference to the Context, and Entity creation also requires the
address of a vault.

```rust
use ockam::Entity;
...
let vault = Vault::create(&ctx)?;
let alice = Entity::create(&ctx, &vault)?;
```

## Profiles

A Profile is a specific identifier backed by a key pair. An Entity can have
multiple Profiles, by having multiple key pairs in the Vault.

The ability for an Entity to have multiple Profiles enhances the privacy of
an Entity. Two Profiles belonging to an Entity cannot be associated with one
another, or back to the Entity. This allows a single real user to use multiple
Profiles, each for a different identity scenario.

For example, a user may have a Manufacturer Identity for technical support, and
an Advertiser Identity for third party integrations.

Entities and Profiles implement the same APIs. In many Ockam APIs, Entities and
Profiles can be used interchangeably.

An Entity has a default Profile which is created automatically. The
`current_profile` function returns this profile. Profiles are created by
calling `create_profile`, and removed with `remove_profile`.

```rust
// Create an Entity
let mut alice = Entity::create(&ctx)?;

// Get the default profile
let alice_default_profile = alice.current_profile().unwrap();

// Create a new profile for chatting with Bob
let alice_chat = alice.create_profile()?;

// Delete a profile
alice.remove_profile(alice_chat.identifier()?)?;
```
