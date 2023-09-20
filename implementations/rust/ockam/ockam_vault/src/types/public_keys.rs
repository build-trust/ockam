use minicbor::{Decode, Encode};

/// X25519 public key length.
pub const X25519_PUBLIC_KEY_LENGTH: usize = 32;

/// Ed25519 public key length.
pub const EDDSA_CURVE25519_PUBLIC_KEY_LENGTH: usize = 32;

/// NIST P256 public key length.
pub const ECDSA_SHA256_CURVEP256_PUBLIC_KEY_LENGTH: usize = 65;

/// A public key for verifying signatures.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub enum VerifyingPublicKey {
    /// Curve25519 Public Key for verifying EdDSA signatures.
    #[n(0)] EdDSACurve25519(#[n(0)] EdDSACurve25519PublicKey),
    /// Curve P-256 Public Key for verifying ECDSA SHA256 signatures.
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256PublicKey),
}

/// A Curve25519 Public Key that is only used for EdDSA signatures.
///
/// - EdDSA Signature as defined [here][1] and [here][2].
/// - Curve25519 as defined [here][3].
///
/// [1]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf
/// [2]: https://ed25519.cr.yp.to/papers.html
/// [2]: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-186.pdf
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cbor(transparent)]
pub struct EdDSACurve25519PublicKey(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; EDDSA_CURVE25519_PUBLIC_KEY_LENGTH],
);

/// A Curve P-256 Public Key that is only used for ECDSA SHA256 signatures.
///
/// This type only supports the uncompressed form which is 65 bytes and has
/// the first byte - 0x04. The uncompressed form is defined [here][1] in
/// section 2.3.3.
///
/// - ECDSA Signature as defined [here][2].
/// - SHA256 as defined [here][3].
/// - Curve P-256 as defined [here][4].
///
/// [1]: https://www.secg.org/SEC1-Ver-1.0.pdf
/// [2]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf
/// [3]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf
/// [4]: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-186.pdf
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cbor(transparent)]
pub struct ECDSASHA256CurveP256PublicKey(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; ECDSA_SHA256_CURVEP256_PUBLIC_KEY_LENGTH],
);

/// X25519 Public Key is used for ECDH.
///
/// - X25519 as defined [here][1].
/// - Curve25519 as defined [here][2].
///
/// [1]: https://datatracker.ietf.org/doc/html/rfc7748
/// [2]: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-186.pdf
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cbor(transparent)]
pub struct X25519PublicKey(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; X25519_PUBLIC_KEY_LENGTH],
);
