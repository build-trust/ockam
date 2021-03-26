use crate::{AsymmetricVault, PublicKey, Secret, VaultRequestMessage, VaultResponseMessage, SecretVault, SecretAttributes, SecretKey};
use ockam_core::{Address, Result, Route, Runner};
use zeroize::Zeroize;
use ockam_node::{Context, block_future};
use rand::random;
use tracing::info;

pub struct VaultRunner {
    ctx: Option<Context>,
    vault_worker_address: Address,
}

impl VaultRunner {
    pub fn start_another(&self) -> Result<Self> {
        let vault_worker_address = self.vault_worker_address.clone();
        let ctx = self.get_ctx()?;
        let runtime = ctx.runtime();

        let clone = block_future(&runtime, async move {
            VaultRunner::start(ctx, vault_worker_address).await.unwrap()
        });

        Ok(clone)
    }
}

impl Zeroize for VaultRunner {
    fn zeroize(&mut self) {}
}

impl VaultRunner {
    pub fn new(vault_worker_address: Address) -> Self {
        Self {
            ctx: None,
            vault_worker_address,
        }
    }

    pub async fn start(ctx: &Context, vault_worker_address: Address) -> Result<Self> {
        let address: Address = random();

        let mut runner = Self::new(vault_worker_address);
        ctx.start_runner(address.clone(), &mut runner).await?;

        Ok(runner)
    }

    fn get_ctx(&self) -> Result<&Context> {
        let ctx;
        if let Some(c) = self.ctx.as_ref() {
            ctx = c;
        } else {
            unimplemented!()
        }

        Ok(ctx)
    }

    fn get_ctx_mut(&mut self) -> Result<&mut Context> {
        let ctx;
        if let Some(c) = self.ctx.as_mut() {
            ctx = c;
        } else {
            unimplemented!()
        }

        Ok(ctx)
    }
}

impl Runner for VaultRunner {
    type Context = Context;

    fn set_ctx(&mut self, ctx: Self::Context) {
        self.ctx = Some(ctx);
    }
}

impl AsymmetricVault for VaultRunner {
    fn ec_diffie_hellman(&mut self, context: &Secret, peer_public_key: &[u8]) -> Result<Secret> {
        let vault_worker_address = self.vault_worker_address.clone();
        let ctx = self.get_ctx_mut()?;
        let runtime = ctx.runtime();

        let s = block_future(&runtime, async {
            // TODO: Add request_id
            ctx.send_message(
                Route::new().append(vault_worker_address),
                VaultRequestMessage::EcDiffieHellman {
                    context: context.clone(),
                    peer_public_key: PublicKey::new(peer_public_key.into()),
                },
            )
            .await.unwrap();

            let resp = ctx
                .receive::<VaultResponseMessage>()
                .await
                .unwrap()
                .take()
                .take();

            if let VaultResponseMessage::EcDiffieHellman(s) = resp {
                s
            } else {
                unimplemented!()
            }
        });

        Ok(s)
    }
}

impl SecretVault for VaultRunner {
    fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret> {
        let vault_worker_address = self.vault_worker_address.clone();
        let ctx = self.get_ctx_mut()?;
        let runtime = ctx.runtime();

        info!("A");

        let s = block_future(&runtime, async move {
            info!("B");
            // TODO: Add request_id
            ctx.send_message(
                Route::new().append(vault_worker_address),
                VaultRequestMessage::SecretGenerate {
                    attributes
                },
            )
                .await.unwrap();

            let resp = ctx
                .receive::<VaultResponseMessage>()
                .await
                .unwrap()
                .take()
                .take();

            if let VaultResponseMessage::SecretGenerate(s) = resp {
                s
            } else {
                unimplemented!()
            }
        });

        Ok(s)
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret> {
        unimplemented!()
    }

    fn secret_export(&mut self, context: &Secret) -> Result<SecretKey> {
        unimplemented!()
    }

    fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes> {
        unimplemented!()
    }

    fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey> {
        let vault_worker_address = self.vault_worker_address.clone();
        let ctx = self.get_ctx_mut()?;
        let runtime = ctx.runtime();

        let s = block_future(&runtime, async {
            // TODO: Add request_id
            ctx.send_message(
                Route::new().append(vault_worker_address),
                VaultRequestMessage::SecretPublicKeyGet {
                    context: context.clone(),
                },
            )
                .await.unwrap();

            let resp = ctx
                .receive::<VaultResponseMessage>()
                .await
                .unwrap()
                .take()
                .take();

            if let VaultResponseMessage::SecretPublicKeyGet(s) = resp {
                s
            } else {
                unimplemented!()
            }
        });

        Ok(s)
    }

    fn secret_destroy(&mut self, context: Secret) -> Result<()> {
        unimplemented!()
    }
}
