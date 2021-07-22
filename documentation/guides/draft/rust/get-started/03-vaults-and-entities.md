# Step 3 - Vaults and Entities

[[Entity]] [[Identity API]] [[Profile]] [[Vault]]


## Vaults

To support a variety of security hardware, there is loose coupling between Ockam security protocols' building blocks and the underlying specific hardware implementation. This is achieved using an abstract notion called Vault.

Vault provides storage for secrets and implements commonly needed operations:
- Generation and storage of public and private key pairs
- Elliptic-curve signing and verifying
- AES-GCM encryption and decryption
- Elliptic-curve Diffie Hellman
- HMAC, SHA-256

An implementation of Vault that uses software for all operations is available as the `SoftwareVault` type.

### Using Vaults

Vaults can store several different kinds of secrets that vary in algorithm, persistence and use case. 

```rust
let mut vault = SoftwareVault::new();
let secret = vault.secret_generate(SecretAttributes::new(
    SecretType::Curve25519,
    SecretPersistence::Ephemeral,
    CURVE25519_SECRET_LENGTH,
))?;
println!("{:?}", secret);
ctx.stop().await
```

Running this example prints:
`Secret { index: 1 }`

In the Vault API, secrets returned to the user are simply index references into the vault. The actual keys can be retrieved using this secret. The below code will retrieve and print the Curve25519 key generated above.

```rust
let public = vault.secret_public_key_get(&secret)?;  
println!("{:?}", public);
```

Other cryptographic primitives are available in the Vault API, for example:

```rust
let mut vault = SoftwareVault::new();  
  
// SHA-256  
let h = vault.sha256("bob".as_bytes())?;  
assert_eq!(h[0..8], [129, 182, 55, 216, 252, 210, 198, 218]);  
  
// AES  
let secret = vault.secret_generate(SecretAttributes::new(  
    SecretType::Aes,  
 SecretPersistence::Ephemeral,  
 AES256_SECRET_LENGTH,  
))?;  
  
let encrypted = vault.aead_aes_gcm_encrypt(  
    &secret,  
 "plaintext".as_bytes(),  
 "nonce value.".as_bytes(),  
 "aad".as_bytes(),  
)?;  
  
let decrypted = vault.aead_aes_gcm_decrypt(  
    &secret,  
 encrypted.as_slice(),  
 "nonce value.".as_bytes(),  
 "aad".as_bytes(),  
)?;  
assert_eq!("plaintext".as_bytes().to_vec(), decrypted);
```


## Vault Worker

In most cases, working with Vault directly is not neccesary. Instead, the system creates a Vault worker. The address of this Vault worker is then given to other local workers.

Creating a Vault worker is done via vault `Vault::create`:

```rust
// Returns the address of a newly started vault worker
let vault = Vault::create(&ctx)?;
```


Vault workers are used by all other workers in a node to access secrets and perform cyptographic operations.

## Entities

The primary worker which uses the vault worker on behalf of a user is called an Entity worker. Entities offer a small, simplified interface to more complex security protocols. They provide the features of the underlying protocols, while handling implementation details.  The interaction between multiple parties establishing trust is modeled using Entities.

Entities provide:

- Cryptographic key creation, rotation and retrieval
- Cryptographic proof creation and verification mechanism
- Secure Channel establishment
- Credential issuance and verification
- Change verification
- Contact management


Like most things in Ockam, an Entity is a worker. 

An Entity is created by calling the `Entity::create` function, with the address of a vault.

```rust
use ockam::Entity;
...
let vault = Vault::create(&ctx)?;
let alice = Entity::create(&ctx, &vault)?;
```


Entities contain their own state and secrets.  The identity that an Entity represents is based on its cryptographic key pair stored in the vault.

This identifier is called the Profile Identifier. 

## Profiles

A Profile is a specific identifier backed by a keypair. An Entity can have multiple Profiles, by having multiple keypairs in the Vault.

The ability for an Entity to have multiple Profiles enhances the privacy of an Entity. Two Profiles belonging to an Entity cannot be associated with one another, or back to the Entity. This allows a single real user to use multiple Profiles, each for a different identity scenario.

For example, a user may have a Manufacturer Identity for technical support, and an Advertiser Identity for third party integrations.

Entities and Profiles implement the same APIs. In many Ockam APIs, Entities and Profiles can be used interchangeably. 

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

Entities and their profiles exchange messages between one another to establish trust, verify identity, and communicate securely. 
