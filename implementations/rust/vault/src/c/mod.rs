use crate::error::VaultFailError;
use crate::types::{PublicKey, SecretKey, SecretKeyAttributes, SecretKeyContext};
use c_bindings::*;
use std::cell::Cell;
use zeroize::Zeroize;

// TODO: Should be thread-safe?
/// Represents a single instance of an Ockam vault context
#[derive(Debug)]
pub struct CVault {
    context: Cell<ockam_vault_t>,
}

impl CVault {
    fn new(vault: ockam_vault_t) -> Self {
        CVault {
            context: Cell::new(vault),
        }
    }
}
