/// EdDSA Ed25519 Signature
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Ed25519Signature(pub [u8; 64]);

/// ECDSA P256 Signature
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct P256ECDSASignature(pub [u8; 64]);
