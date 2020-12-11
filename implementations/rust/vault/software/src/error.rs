use ockam_vault::error::{VaultFailError, VaultFailErrorKind};

// FIXME: This should be removed after introducing common error

pub(crate) fn map_hkdf_invalid_length_err(_: hkdf::InvalidLength) -> VaultFailError {
    VaultFailError::from(VaultFailErrorKind::HkdfSha256)
}

pub(crate) fn map_aes_error(_: aes_gcm::Error) -> VaultFailError {
    VaultFailError::from(VaultFailErrorKind::AeadAesGcm)
}
