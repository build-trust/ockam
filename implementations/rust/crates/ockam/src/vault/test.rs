use super::*;

#[test]
fn can_generate_random_number() {
    use rand::prelude::*;

    let mut vault = Vault::new().unwrap();

    let a: u32 = vault.gen();
    let b: u32 = vault.gen();

    assert_ne!(a, b);
}
