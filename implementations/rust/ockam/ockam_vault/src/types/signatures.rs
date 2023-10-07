use minicbor::{Decode, Encode};

/// EdDSACurve25519 signature length.
pub const EDDSA_CURVE25519_SIGNATURE_LENGTH: usize = 64;
/// ECDSASHA256CurveP256 signature length.
pub const ECDSA_SHA256_CURVEP256_SIGNATURE_LENGTH: usize = 64;

/// A cryptographic signature.
#[derive(Encode, Decode)]
#[rustfmt::skip]
pub enum Signature {
    /// An EdDSA signature using Curve 25519.
    #[n(0)] EdDSACurve25519(#[n(0)] EdDSACurve25519Signature),
    /// An ECDSA signature using SHA-256 and Curve P-256.
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256Signature),
}

/// An EdDSA Signature using Curve25519.
///
/// - EdDSA Signature as defined [here][1].
/// - Curve25519 as defined in [here][2].
///
/// [1]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf
/// [2]: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-186.pdf
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
#[cbor(transparent)]
pub struct EdDSACurve25519Signature(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; EDDSA_CURVE25519_SIGNATURE_LENGTH],
);

/// An ECDSA Signature using SHA256 and Curve P-256.
///
/// - ECDSA Signature as defined [here][1].
/// - SHA256 as defined [here][2].
/// - Curve P-256 as defined [here][3].
///
/// [1]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf
/// [2]: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf
/// [3]: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-186.pdf
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
#[cbor(transparent)]
pub struct ECDSASHA256CurveP256Signature(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; ECDSA_SHA256_CURVEP256_SIGNATURE_LENGTH],
);
