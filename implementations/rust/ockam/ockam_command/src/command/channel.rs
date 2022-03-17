use crate::service::echoer::ECHOER_SERVICE_NAME;
use crate::AppError;
use log::error;
use ockam::{
    route, Context, Identity, IdentityTrait, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};

pub struct ChannelCommand {}

impl ChannelCommand {
    pub async fn run(
        ctx: &mut Context,
        secret_key_path: String,
        channel_address: &str,
        channel_name: &str,
        message: &str,
    ) -> Result<(), AppError> {
        let _tcp = TcpTransport::create(ctx).await?;

        let vault = Vault::create();
        let alice = Identity::create(ctx, &vault).await?;

        let secret_key = std::fs::read_to_string(secret_key_path)?;

        let secret_key = ssh_key::PrivateKey::from_openssh(&secret_key)?;
        let secret_key = secret_key.key_data.ed25519();

        if secret_key.is_none() {
            // TODO: This error is getting cut off when printed. It also seems to randomly not print at all
            error!("Only Ed25519 SSH keys are currently supported.");
            return Ok(());
        }

        let secret_key = secret_key.unwrap().private.clone();

        let secret_key = vault
            .secret_import(
                secret_key.as_ref(),
                SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Ephemeral, 32),
            )
            .await?;

        alice.add_key("SSH".into(), &secret_key).await?;

        let channel = alice
            .create_secure_channel(
                route![(TCP, channel_address), channel_name],
                TrustEveryonePolicy,
            )
            .await?;

        ctx.send(route![channel, ECHOER_SERVICE_NAME], message.to_string())
            .await?;
        let msg = ctx.receive::<String>().await?.take().body();
        println!("Echo back: {}", &msg);
        Ok(())
    }
}
