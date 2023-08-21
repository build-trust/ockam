/// SHA256 digest length
pub const SHA256_LENGTH: usize = 32;

/// X25519 private key length.
pub const X25519_SECRET_LENGTH_U32: u32 = 32;
/// X25519 private key length.
pub const X25519_SECRET_LENGTH_USIZE: usize = 32;

/// X25519 public key length.
pub const X25519_PUBLIC_LENGTH_U32: u32 = 32;
/// X25519 public key length.
pub const X25519_PUBLIC_LENGTH_USIZE: usize = 32;

/// Ed25519 private key length.
pub const ED25519_SECRET_LENGTH_U32: u32 = 32;
/// Ed25519 private key length.
pub const ED25519_SECRET_LENGTH_USIZE: usize = 32;

/// Ed25519 public key length.
pub const ED25519_PUBLIC_LENGTH_U32: u32 = 32;
/// Ed25519 public key length.
pub const ED25519_PUBLIC_LENGTH_USIZE: usize = 32;

/// Ed25519 signature length.
pub const ED25519_SIGNATURE_LENGTH_USIZE: usize = 64;

/// NIST P256 private key length.
pub const NIST_P256_SECRET_LENGTH_U32: u32 = 32;
/// NIST P256 private key length.
pub const NIST_P256_SECRET_LENGTH_USIZE: usize = 32;

/// NIST P256 public key length.
pub const NIST_P256_PUBLIC_LENGTH_U32: u32 = 65;
/// NIST P256 public key length.
pub const NIST_P256_PUBLIC_LENGTH_USIZE: usize = 65;

/// NIST P256 signature length.
pub const NIST_P256_SIGNATURE_LENGTH_USIZE: usize = 64;

/// AES256 private key length.
pub const AES256_SECRET_LENGTH_U32: u32 = 32;
/// AES256 private key length.
pub const AES256_SECRET_LENGTH_USIZE: usize = 32;

/// AES128 private key length.
pub const AES128_SECRET_LENGTH_U32: u32 = 16;
/// AES128 private key length.
pub const AES128_SECRET_LENGTH_USIZE: usize = 16;

use static_assertions::const_assert_eq;

const_assert_eq!(X25519_SECRET_LENGTH_U32, X25519_SECRET_LENGTH_USIZE as u32);
const_assert_eq!(X25519_PUBLIC_LENGTH_U32, X25519_PUBLIC_LENGTH_USIZE as u32);
const_assert_eq!(
    ED25519_SECRET_LENGTH_U32,
    ED25519_SECRET_LENGTH_USIZE as u32
);
const_assert_eq!(
    ED25519_PUBLIC_LENGTH_U32,
    ED25519_PUBLIC_LENGTH_USIZE as u32
);
const_assert_eq!(
    NIST_P256_SECRET_LENGTH_U32,
    NIST_P256_SECRET_LENGTH_USIZE as u32
);
const_assert_eq!(
    NIST_P256_PUBLIC_LENGTH_U32,
    NIST_P256_PUBLIC_LENGTH_USIZE as u32
);
const_assert_eq!(AES256_SECRET_LENGTH_U32, AES256_SECRET_LENGTH_USIZE as u32);
const_assert_eq!(AES128_SECRET_LENGTH_U32, AES128_SECRET_LENGTH_USIZE as u32);
