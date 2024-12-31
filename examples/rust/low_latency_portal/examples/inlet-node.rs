use log::info;
use low_latency_portal::{parse, InletConfig};
use ockam::identity::models::ChangeHistory;
use ockam::identity::{Identifier, SecureChannelOptions, SecureChannels, TrustIdentifierPolicy, Vault};
use ockam::tcp::{TcpConnectionOptions, TcpInletOptions, TcpTransport};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{route, Context, Result};
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    info!("A");
    let config = if let Some(config) = args.get(1) {
        config.clone()
    } else {
        let config = std::fs::read("inlet.config.json").unwrap();
        String::from_utf8(config).unwrap()
    };

    info!("B");
    let config: InletConfig = parse(&config)?;
    info!("C");

    let outlet_identifier = Identifier::from_str(&config.outlet_identifier)?;
    let inlet_identity_key = hex::decode(config.inlet_identity_key).unwrap();
    let inlet_change_history = ChangeHistory::import_from_string(&config.inlet_change_history)?;

    info!("D");

    let tcp = TcpTransport::create(&ctx).await?;

    info!("E");

    let identity_vault = SoftwareVaultForSigning::create().await?; // FIXME: 16ms

    info!("F");

    let relay_identity_key = EdDSACurve25519SecretKey::new(inlet_identity_key.try_into().unwrap());

    info!("G");
    let relay_identity_key = SigningSecret::EdDSACurve25519(relay_identity_key);

    info!("H");

    identity_vault.import_key(relay_identity_key).await?;

    info!("J");

    let mut vault = Vault::create().await?; // FIXME: 25ms
    vault.identity_vault = identity_vault;

    info!("K");

    let secure_channels = SecureChannels::builder().await?.with_vault(vault).build(); // FIXME: 16 ms

    info!("L");

    let inlet_identifier = secure_channels
        .identities()
        .identities_verification()
        .import_from_change_history(None, inlet_change_history)
        .await?;

    info!("M");

    let tcp_connection_to_relay = tcp
        .connect(config.relay_address.to_string(), TcpConnectionOptions::new())
        .await?;

    info!("N");

    let secure_channel_to_outlet = secure_channels
        .create_secure_channel(
            &ctx,
            &inlet_identifier,
            route![
                tcp_connection_to_relay,
                format!("forward_to_{}", config.outlet_relay_name),
                "api"
            ],
            SecureChannelOptions::new().with_trust_policy(TrustIdentifierPolicy::new(outlet_identifier)),
        )
        .await?;

    info!("O");

    tcp.create_inlet(
        config.inlet_address.to_string(),
        route![secure_channel_to_outlet, "outlet"],
        TcpInletOptions::new(),
    )
    .await?;

    info!("P");

    info!("Initialized successfully");

    ctx.stop().await?;

    info!("Q");

    Ok(())
}
