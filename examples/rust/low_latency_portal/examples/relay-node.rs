use log::info;
use low_latency_portal::{parse, RelayConfig};
use ockam::identity::models::ChangeHistory;
use ockam::identity::{Identifier, SecureChannelListenerOptions, SecureChannels, TrustIdentifierPolicy, Vault};
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{Context, RelayService, RelayServiceOptions, Result};
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let config = if let Some(config) = args.get(1) {
        config.clone()
    } else {
        let config = std::fs::read("relay.config.json").unwrap();
        String::from_utf8(config).unwrap()
    };

    let config: RelayConfig = parse(&config)?;

    let outlet_identifier = Identifier::from_str(&config.outlet_identifier)?;
    let relay_identity_key = hex::decode(config.relay_identity_key).unwrap();
    let relay_change_history = ChangeHistory::import_from_string(&config.relay_change_history)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let identity_vault = SoftwareVaultForSigning::create().await?;
    let relay_identity_key = EdDSACurve25519SecretKey::new(relay_identity_key.try_into().unwrap());
    let relay_identity_key = SigningSecret::EdDSACurve25519(relay_identity_key);
    identity_vault.import_key(relay_identity_key).await?;

    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let secure_channels = SecureChannels::builder().await?.with_vault(vault).build();

    let relay_identifier = secure_channels
        .identities()
        .identities_verification()
        .import_from_change_history(None, relay_change_history)
        .await?;

    let tcp_listener_options = TcpListenerOptions::new();
    let secure_channel_listener_options = SecureChannelListenerOptions::new()
        .as_consumer(&tcp_listener_options.spawner_flow_control_id())
        .with_trust_policy(TrustIdentifierPolicy::new(outlet_identifier));
    let relay_service_options = RelayServiceOptions::new()
        .service_as_consumer(&secure_channel_listener_options.spawner_flow_control_id())
        .relay_as_consumer(&tcp_listener_options.spawner_flow_control_id())
        .prefix("forward_to_");

    RelayService::create(&ctx, "static_forwarding_service", relay_service_options).await?;

    secure_channels
        .create_secure_channel_listener(&ctx, &relay_identifier, "api", secure_channel_listener_options)
        .await?;

    tcp.listen(config.relay_listener_address.to_string(), tcp_listener_options)
        .await?;

    info!("Initialized successfully");

    Ok(())
}
