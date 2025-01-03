use log::info;
use low_latency_portal::{parse, OutletConfig};
use ockam::identity::models::ChangeHistory;
use ockam::identity::{
    Identifier, SecureChannelListenerOptions, SecureChannelOptions, SecureChannels, TrustIdentifierPolicy,
    TrustMultiIdentifiersPolicy, Vault,
};
use ockam::remote::{RemoteRelay, RemoteRelayOptions};
use ockam::tcp::{TcpConnectionOptions, TcpOutletOptions, TcpTransport};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{route, Context, Result};
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let config = if let Some(config) = args.get(1) {
        config.clone()
    } else {
        let config = std::fs::read("outlet.config.json").unwrap();
        String::from_utf8(config).unwrap()
    };

    let config: OutletConfig = parse(&config)?;

    if config.tls == Some(true) {
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install aws-lc crypto provider");
    }

    let relay_identifier = Identifier::from_str(&config.relay_identifier)?;
    let outlet_identity_key = hex::decode(config.outlet_identity_key).unwrap();
    let inlet_identifiers = config
        .inlet_identifiers
        .iter()
        .map(|i| Identifier::from_str(i).unwrap())
        .collect();
    let outlet_change_history = ChangeHistory::import_from_string(&config.outlet_change_history)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let identity_vault = SoftwareVaultForSigning::create().await?;
    let outlet_identity_key = EdDSACurve25519SecretKey::new(outlet_identity_key.try_into().unwrap());
    let outlet_identity_key = SigningSecret::EdDSACurve25519(outlet_identity_key);
    identity_vault.import_key(outlet_identity_key).await?;

    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let secure_channels = SecureChannels::builder().await?.with_vault(vault).build();

    let outlet_identifier = secure_channels
        .identities()
        .identities_verification()
        .import_from_change_history(None, outlet_change_history)
        .await?;

    let tcp_connection_options = TcpConnectionOptions::new();
    let secure_channel_options =
        SecureChannelOptions::new().with_trust_policy(TrustIdentifierPolicy::new(relay_identifier));
    let secure_channel_listener_options = SecureChannelListenerOptions::new()
        .as_consumer(&secure_channel_options.producer_flow_control_id())
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(inlet_identifiers));
    let tcp_outlet_options = TcpOutletOptions::new()
        .as_consumer(&secure_channel_listener_options.spawner_flow_control_id())
        .with_tls(config.tls.unwrap_or(false));

    tcp.create_outlet("outlet", config.outlet_peer_address, tcp_outlet_options)
        .await?;

    secure_channels
        .create_secure_channel_listener(&ctx, &outlet_identifier, "api", secure_channel_listener_options)
        .await?;

    let tcp_connection_to_relay = tcp
        .connect(config.relay_address.to_string(), tcp_connection_options)
        .await?;
    let secure_channel_to_relay = secure_channels
        .create_secure_channel(
            &ctx,
            &outlet_identifier,
            route![tcp_connection_to_relay, "api"],
            secure_channel_options,
        )
        .await?;

    let relay_options = RemoteRelayOptions::new();
    RemoteRelay::create_static(
        &ctx,
        route![secure_channel_to_relay],
        config.outlet_relay_name,
        relay_options,
    )
    .await?;

    info!("Initialized successfully");

    Ok(())
}
