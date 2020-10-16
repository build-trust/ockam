use crate::bindings::*;
use zeroize::Zeroize;

pub fn ockam_error_is_none(error: &ockam_error_t) -> bool {
    error.code == 0
}

pub fn ockam_error_has_error(error: &ockam_error_t) -> bool {
    error.code != 0
}

impl Default for ockam_vault_secret_t {
    fn default() -> Self {
        ockam_vault_secret_t {
            attributes: ockam_vault_secret_attributes_t {
                length: 0,
                type_: ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_BUFFER,
                purpose: ockam_vault_secret_purpose_t::OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                persistence: ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_EPHEMERAL,
            },
            context: std::ptr::null_mut(),
        }
    }
}

impl Default for ockam_vault_secret_attributes_t {
    fn default() -> Self {
        ockam_vault_secret_attributes_t {
            length: 0,
            type_: ockam_vault_secret_type_t::OCKAM_VAULT_SECRET_TYPE_BUFFER,
            purpose: ockam_vault_secret_purpose_t::OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
            persistence: ockam_vault_secret_persistence_t::OCKAM_VAULT_SECRET_EPHEMERAL,
        }
    }
}

impl Zeroize for ockam_vault_secret_t {
    fn zeroize(&mut self) {
        unimplemented!()
    }
}
