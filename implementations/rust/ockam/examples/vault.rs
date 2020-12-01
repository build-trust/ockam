use ockam::{
    error::{Result as OckamResult, Error},
    vault::{SecretVault, Secret, SecretKey, PublicKey, SecretAttributes}
};

pub struct TestVault {

}

#[derive(Debug)]
pub struct TestSecret {

}

impl TestSecret {
    fn from_secret(secret: &Box<dyn Secret>) -> OckamResult<&Self> {
        secret.downcast_ref::<TestSecret>().map_err(|_| Error::from_msg(1, "invalid secret context"))
    }
}

impl Secret for TestSecret {
}

impl SecretVault for TestVault {
    fn secret_generate(&mut self, _attributes: SecretAttributes) -> OckamResult<Box<dyn Secret>> {
        unimplemented!()
    }

    fn secret_import(&mut self, _secret: &SecretKey, _attributes: SecretAttributes) -> OckamResult<Box<dyn Secret>> {
        unimplemented!()
    }

    fn secret_public_key_get(&mut self, context: &Box<dyn Secret>) -> OckamResult<PublicKey> {
        let _s = TestSecret::from_secret(context)?;
        unimplemented!()
    }
}

fn main() {

}