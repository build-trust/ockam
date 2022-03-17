use crate::service::echoer::{Echoer, ECHOER_SERVICE_NAME};
use crate::spinner::Spinner;
use crate::AppError;
use comfy_table::Table;
use log::error;
use ockam::{
    AsyncTryClone, Context, Identity, IdentityAccessControlBuilder, TcpTransport,
    TrustPublicKeyPolicy, Vault,
};
use ockam_core::vault::{PublicKey, SecretType};

pub struct ChannelListenCommand {}

impl ChannelListenCommand {
    pub async fn run(
        ctx: &Context,
        public_key_path: String,
        listener_address: &str,
        listener_name: &str,
    ) -> Result<(), AppError> {
        let spinner = Spinner::default();

        let public_key = std::fs::read_to_string(public_key_path)?;

        let public_key = ssh_key::PublicKey::from_openssh(&public_key)?;
        let public_key = public_key.key_data.ed25519();

        if public_key.is_none() {
            // TODO: This error is getting cut off when printed. It also seems to randomly not print at all
            error!("Only Ed25519 SSH keys are currently supported.");
            return Ok(());
        }

        let public_key = public_key.unwrap();

        let public_key = PublicKey::new(public_key.as_ref().to_vec(), SecretType::Ed25519);

        let vault = Vault::create();

        let access_control = IdentityAccessControlBuilder::new_with_any_id();
        ctx.start_worker_with_access_control(ECHOER_SERVICE_NAME, Echoer, access_control)
            .await?;

        let identity = Identity::create(ctx, &vault).await?;

        let trust_policy =
            TrustPublicKeyPolicy::new(public_key, "SSH", identity.async_try_clone().await?);

        identity
            .create_secure_channel_listener(listener_name, trust_policy)
            .await?;

        let tcp = TcpTransport::create(ctx).await?;
        tcp.listen(listener_address).await?;

        spinner.stop("Created Secure Channel listener");

        let mut table = Table::new();
        table
            .set_header(vec!["Secure Channel", "Address"])
            .add_row(vec![listener_name, listener_address]);

        println!("{}", table);

        Ok(())
    }
}
