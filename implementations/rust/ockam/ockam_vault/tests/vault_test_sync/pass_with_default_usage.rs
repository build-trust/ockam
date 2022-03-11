use ockam_vault::SoftwareVault;

fn new_vault() -> SoftwareVault {
    SoftwareVault::default()
}

#[ockam_macros::vault_test_sync]
fn hkdf() {}

fn main() {}
