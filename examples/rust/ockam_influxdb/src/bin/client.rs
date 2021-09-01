use ockam::{route, Context, Entity, Identity, Result, TcpTransport, Vault, TCP};
// TODO: Add these use when we switch to secure channel
// use ockam::{SecureChannels, TrustEveryonePolicy}

use ockam_influxdb::InfluxClient;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx)?;
    let mut entity = Entity::create(&ctx, &vault)?;
    // TODO: Uncomment when secure channel is available on Hub
    // Create a secure channel
    /*
        let secure_channel_route = route![(TCP, "Paste the hostname:port of your Ockam Hub node here."), "secure_channel"];
        let secure_channel = entity.create_secure_channel(secure_channel_route, TrustEveryonePolicy)?;
    */

    // InfluxDB details
    let api_url = "Paste the URL of your InfluxDB instance here.";
    let org = "Paste your 16 digit orgID here.";
    let bucket = "Paste your bucket name here.";
    let ttl = 5_000; // 5 seconds

    // Get an API token from the Token Lease Service
    // TODO: Use this route when secure channel is available
    // let lease_route = route![secure_channel, "influxdb_token_lease_service"];
    let lease_route = route![
        (TCP, "Paste the hostname:port of your Ockam Hub node here."),
        "influxdb_token_lease_service"
    ];

    let leased_token = entity.get_lease(&lease_route, org, bucket, ttl)?;

    // Create the InfluxDB client using the leased token
    let mut influx_client = InfluxClient::new(api_url, org, bucket, leased_token.value());

    // Write data once per second. On authentication failure, request a new lease.
    loop {
        let response = influx_client.send_metrics().await;
        if let Err(influx_error) = response {
            if influx_error.is_authentication_error() {
                println!("Authentication failed. Revoking lease.");
                entity.revoke_lease(&lease_route, leased_token.clone())?;

                // Interactively pause. This allows an opportunity to verify the API token status globally.
                println!("Press enter to get a new lease");
                std::io::stdin().read_exact(&mut [0_u8; 1]).unwrap();

                // Get a new lease
                let leased_token = entity.get_lease(&lease_route, org, bucket, ttl)?;

                // Update the client
                influx_client.set_token(leased_token.value());
            } else {
                panic!("Received an unexpected error.")
            }
        }
        sleep(Duration::from_secs(1));
    }
}
