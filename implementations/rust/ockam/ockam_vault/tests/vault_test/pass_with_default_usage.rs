use ockam_vault::SoftwareVault;

fn new_vault() -> SoftwareVault {
    SoftwareVault::default()
}

#[ockam_macros::vault_test]
fn hkdf() {}

fn main() {}
