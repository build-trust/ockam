use log::info;
use low_latency_portal::{parse, HashMapRepository, InletConfig};
use ockam::compat::str::FromStr;
use ockam::compat::sync::Arc;
use ockam::identity::models::ChangeHistory;
use ockam::identity::{
    Identifier, Identities, SecureChannelOptions, SecureChannelRegistry, SecureChannels, TrustIdentifierPolicy, Vault,
};
use ockam::tcp::{TcpConnectionOptions, TcpInletOptions, TcpTransport};
use ockam::vault::{
    EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSecureChannels, SoftwareVaultForSigning,
    SoftwareVaultForVerifyingSignatures,
};
use ockam::{route, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let config = if let Some(config) = args.get(1) {
        config.clone()
    } else {
        let config = std::fs::read("inlet.config.json").unwrap();
        String::from_utf8(config).unwrap()
    };

    let config: InletConfig = parse(&config)?;

    let outlet_identifier = Identifier::from_str(&config.outlet_identifier)?;
    let inlet_identity_key = hex::decode(config.inlet_identity_key).unwrap();
    let inlet_change_history = ChangeHistory::import_from_string(&config.inlet_change_history)?;

    let hash_map_storage = Arc::new(HashMapRepository::default());

    let tcp = TcpTransport::create(&ctx).await?;

    let identity_vault = Arc::new(SoftwareVaultForSigning::new(hash_map_storage.clone()));
    let secure_channel_vault = Arc::new(SoftwareVaultForSecureChannels::new(hash_map_storage.clone()));
    let credential_vault = Arc::new(SoftwareVaultForSigning::new(hash_map_storage.clone()));
    let verifying_vault = Arc::new(SoftwareVaultForVerifyingSignatures::new());

    let relay_identity_key = EdDSACurve25519SecretKey::new(inlet_identity_key.try_into().unwrap());

    let relay_identity_key = SigningSecret::EdDSACurve25519(relay_identity_key);

    identity_vault.import_key(relay_identity_key).await?;

    let vault = Vault::new(identity_vault, secure_channel_vault, credential_vault, verifying_vault);

    let identities = Identities::new(
        vault,
        hash_map_storage.clone(),
        hash_map_storage.clone(),
        hash_map_storage.clone(),
        hash_map_storage.clone(),
    );
    let secure_channels = SecureChannels::new(
        Arc::new(identities),
        SecureChannelRegistry::new(),
        hash_map_storage.clone(),
    );

    let inlet_identifier = secure_channels
        .identities()
        .identities_verification()
        .import_from_change_history(None, inlet_change_history)
        .await?;

    let tcp_connection_to_relay = tcp
        .connect(config.relay_address.to_string(), TcpConnectionOptions::new())
        .await?;

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

    tcp.create_inlet(
        config.inlet_address.to_string(),
        route![secure_channel_to_outlet, "outlet"],
        TcpInletOptions::new(),
    )
    .await?;

    info!("Initialized successfully");

    Ok(())
}
