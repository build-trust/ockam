use low_latency_portal::{InletConfig, OutletConfig, RelayConfig};
use ockam::compat::rand::{thread_rng, RngCore};
use ockam::identity::{Identities, IdentityBuilder, Vault};
use ockam::transport::HostnamePort;
use ockam::vault::{
    EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning, EDDSA_CURVE25519_SECRET_KEY_LENGTH,
};
use ockam::{Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let identity_vault = SoftwareVaultForSigning::create().await?;

    let mut inlet_key_binary = [0u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH];
    let mut outlet_key_binary = [0u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH];
    let mut relay_key_binary = [0u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH];

    {
        let mut rng = thread_rng();

        rng.fill_bytes(&mut inlet_key_binary);
        rng.fill_bytes(&mut outlet_key_binary);
        rng.fill_bytes(&mut relay_key_binary);
    }

    let inlet_key = identity_vault
        .import_key(SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new(
            inlet_key_binary,
        )))
        .await?;
    let outlet_key = identity_vault
        .import_key(SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new(
            outlet_key_binary,
        )))
        .await?;
    let relay_key = identity_vault
        .import_key(SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new(
            relay_key_binary,
        )))
        .await?;

    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let identities = Identities::builder().await?.with_vault(vault).build();
    let inlet_identifier = IdentityBuilder::new(identities.identities_creation())
        .with_existing_key(inlet_key)
        .build()
        .await?;
    let outlet_identifier = IdentityBuilder::new(identities.identities_creation())
        .with_existing_key(outlet_key)
        .build()
        .await?;
    let relay_identifier = IdentityBuilder::new(identities.identities_creation())
        .with_existing_key(relay_key)
        .build()
        .await?;

    let inlet_config = InletConfig {
        inlet_change_history: hex::encode(identities.export_identity(&inlet_identifier).await?),
        inlet_identity_key: hex::encode(inlet_key_binary),
        inlet_address: HostnamePort::new("0.0.0.0", 4000),
        relay_address: HostnamePort::new("127.0.0.1", 4001),
        outlet_identifier: outlet_identifier.to_string(),
        outlet_relay_name: "outlet_relay".to_string(),
    };

    let outlet_config = OutletConfig {
        outlet_change_history: hex::encode(identities.export_identity(&outlet_identifier).await?),
        outlet_identity_key: hex::encode(outlet_key_binary),
        outlet_relay_name: "outlet_relay".to_string(),
        outlet_peer_address: HostnamePort::new("127.0.0.1", 5000),
        relay_identifier: relay_identifier.to_string(),
        relay_address: HostnamePort::new("127.0.0.1", 4001),
        inlet_identifiers: vec![inlet_identifier.to_string()],
    };

    let relay_config = RelayConfig {
        relay_change_history: hex::encode(identities.export_identity(&relay_identifier).await?),
        relay_identity_key: hex::encode(relay_key_binary),
        outlet_identifier: outlet_identifier.to_string(),
        relay_listener_address: HostnamePort::new("0.0.0.0", 4001),
    };

    std::fs::write("inlet.config.json", serde_json::to_vec(&inlet_config).unwrap()).unwrap();
    std::fs::write("outlet.config.json", serde_json::to_vec(&outlet_config).unwrap()).unwrap();
    std::fs::write("relay.config.json", serde_json::to_vec(&relay_config).unwrap()).unwrap();

    ctx.stop().await
}
