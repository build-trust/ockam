mod configs;
mod hash_map_repository;

pub use configs::*;
pub use hash_map_repository::*;

use crate::HashMapRepository;
use log::info;
use ockam::abac::tokio;
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
use ockam::{route, NodeBuilder};
use std::net::SocketAddr;

pub fn run_inlet(config: Option<String>, callback_address: Option<SocketAddr>) {
    let (ctx, mut executor) = NodeBuilder::new().build();
    executor
        .execute(async move {
            let config = config.unwrap_or_else(|| {
                let config = std::fs::read("inlet.config.json").unwrap();
                String::from_utf8(config).unwrap()
            });

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

            if let Some(callback_address) = callback_address {
                let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();

                socket.send_to(&[], callback_address).await.unwrap();

                info!("Sent callback signal");
            }

            Ok::<(), ockam::Error>(())
        })
        .unwrap()
        .unwrap();
}
