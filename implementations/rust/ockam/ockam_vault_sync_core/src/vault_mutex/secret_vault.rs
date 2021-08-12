use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret, SecretAttributes, SecretKey, SecretVault};

impl<V: SecretVault> SecretVault for VaultMutex<V> {
    fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_generate(attributes);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_generate(attributes)
        });
    }

    fn secret_import(&mut self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_import(secret, attributes);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_import(secret, attributes)
        });
    }

    fn secret_export(&mut self, context: &Secret) -> Result<SecretKey> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_export(context);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_export(context)
        });
    }

    fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_attributes_get(context);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_attributes_get(context)
        });
    }

    fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_public_key_get(context);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_public_key_get(context)
        });
    }

    fn secret_destroy(&mut self, context: Secret) -> Result<()> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().secret_destroy(context);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .secret_destroy(context)
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::VaultMutex;
    use ockam_vault::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> VaultMutex<SoftwareVault> {
        VaultMutex::create(SoftwareVault::default())
    }

    #[vault_test]
    fn new_public_keys() {}

    #[vault_test]
    fn new_secret_keys() {}

    #[vault_test]
    fn secret_import_export() {}

    #[vault_test]
    fn secret_attributes_get() {}
}
