use ockam_vault::Vault;

fn new_vault() -> Vault {
    Vault::default()
}

#[ockam_macros::vault_test]
fn hkdf() {}

fn main() {}
