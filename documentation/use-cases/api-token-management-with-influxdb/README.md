# Using Leases to manage InfluxDB API tokens

In this guide we'll show how Ockam can be used to securely distribute and manage InfluxDB API Tokens with access control
features like revocation and expiration.

The Ockam Lease Manager is a service that runs in the Ockam Hub. A lease is like an envelope around an API token, adding
additional authorization information.The Lease Manager InfluxDB integration enables a Cloud Node to create, update and
revoke API tokens for an InfluxDB instance.

Devices in the field obtain InfluxDB API tokens from the Lease Manager using a secure channel. When the lease expires, or
the underlying API token has been revoked, the device can request a new lease.

This dynamic provisioning of API tokens allows scalable and secure management of credentials over time. If an API token
becomes compromised, it can simply be revoked. This also prevents the common security issue of deploying hard-coded API
tokens.

# Example

Let's build a Rust program that sends data to InfluxDB using a leased API token. We will:

- set up an Ockam Cloud Node and configure the InfluxDB integration
- create an Ockam Node in Rust for the device
- establish a secure channel between the Device Node to the Cloud Node
- obtain a lease for an API token
- send data from the Node to an InfluxDB instance using the leased API token
- demonstrate lease expiration

## Ockam Cloud Setup

- Login to [Ockam Hub](https://hub.ockam.network/) using your GitHub Credentials.
- Click `Create Custom InfluxDB Node`.

You will see a form with fields for the InfluxDB instance details. If you have an InfluxDB instance that is reachable
over the internet, you can enter its endpoint details here.

- Enter the connection details for the host and port of your instance.
- The orgID is your 16 digit organization number on the instance. This is present in most InfluxDB URLs.
- Paste your instance's "All Access"/Operator Token in the token field.

### Using a test Influx instance

If you do not have an InfluxDB instance available for testing, you can run this demo by using the details of an InfluxDB
instance we have provisioned for this guide.

To use the Ockam Influx instance:
- Use `ad86b57ea89544295acb747a69c7afd9-345558404.us-west-1.elb.amazonaws.com` as the InfluxDB host.
- Use '8086' as the InfluxDB port.
- Use `217eba198af721b8` as the `orgID`
- Use `In79oyAHqJZt74Sf0FuqqF6ERKQ_MD9ANj2DCTWbp-eLPTjOM87hmGAyTg-_F4m-jJ-Z-ZPHRsBNigdodsa3zg==` as the token.
- Use 'my-bucket' as the bucket parameter, in the client code below.

After clicking create, an Ockam Node will be provisioned. This process can take a few minutes. When the instance is
available, its status will change to `Ready` in the Ockam Hub UI.

## Rust Setup

If you don't have it, please [install](https://www.rust-lang.org/tools/install) the latest version of Rust.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Next, create a new cargo project to get started:

```
cargo new --lib ockam_influxdb && cd ockam_influxdb
```

Add the following dependencies:

```toml
[dependencies]
ockam = "0"
ockam_transport_tcp = "0"
ockam_entity = { version = "0", features = ["lease_proto_json"] }
reqwest = { version = "0", features = ["json"] }
rand = "0"
```

## InfluxDB Client

In this example, we will use the `reqwest` crate to build a small InfluxDB v2.0 HTTPS client. This client writes 10
random values into a `metrics` measurement.

Create a new file named `src/influx_client.rs`.  Paste the below content into the file:

```rust
use rand::random;
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Represents potential InfluxDB errors. Specifically, we are interested in categorizing authentication
/// errors distinctly from other errors. This allows us to take specific actions, such as revoking a lease.
#[derive(Debug, Clone)]
pub enum InfluxError {
    Authentication,
    Unknown,
}

impl Display for InfluxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Something went wrong.")
    }
}

impl Error for InfluxError {}

impl InfluxError {
    pub fn is_authentication_error(&self) -> bool {
        matches!(self, InfluxError::Authentication)
    }
}

/// A basic InfluxDB client. Contains InfluxDB meta-data and a leased token.
pub struct InfluxClient {
    api_url: String,
    org: String,
    bucket: String,
    leased_token: String,
}

impl InfluxClient {
    /// Create a new client.
    pub fn new(api_url: &str, org: &str, bucket: &str, leased_token: &str) -> Self {
        InfluxClient {
            api_url: api_url.to_string(),
            org: org.to_string(),
            bucket: bucket.to_string(),
            leased_token: leased_token.to_string(),
        }
    }

    /// Set the current token.
    pub fn set_token(&mut self, leased_token: &str) {
        self.leased_token = leased_token.to_string();
    }

    /// Send some random metrics to InfluxDB.
    pub async fn send_metrics(&self) -> Result<(), InfluxError> {
        let url = format!(
            "{}/api/v2/write?org={}&bucket={}&precision=s",
            self.api_url, self.org, self.bucket
        );

        let mut headers = HeaderMap::new();
        let token = format!("Token {}", self.leased_token);

        headers.insert(
            "Authorization",
            HeaderValue::from_str(token.as_str()).unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        // Send 10 data points. On authentication error (403), return an `InfluxError::Authentication`
        for i in 0..10 {
            let data = random::<usize>() % 10_000;
            let metric = format!("metrics,env=test r{}={}", i, data);
            let resp = client.post(url.clone()).body(metric).send().await.unwrap();
            let status = resp.status().as_u16();
            if (401..=403).contains(&status) {
                return Err(InfluxError::Authentication);
            }
        }
        Ok(())
    }
}
```

Let's expose this client by re-exporting from `lib.rs`.

Create `src/lib.rs` and paste:

```rust
mod influx_client;
pub use influx_client::*;
```

## Device Node

Now that we have an InfluxDB client to work with, we can use it in a node. The below client is a typical Ockam Node
application. In the application, we:

- Create a Node using the node attribute and async main.
- Use a TCP transport to connect to another Node.
- Create a Vault for secrets management.
- Create an Entity to access Ockam APIs.

In addition to this setup, we take a few additional steps to use the Lease Manager:

- Create a secure channel to the Hub.
- Create a route to the `influxdb_token_lease_service` on the Hub.
- Get a Leased API token from the Hub.

To demonstrate management of leases, we show:

- Lease validity control using time-to-live expiration.
- Detection of authentication errors from InfluxDB API.
- Re-provisioning of a lease after revocation.

The example client requires a small bit of customization to run against your Node and InfluxDB instance:

```rust
    ...
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
    ...
```

We also specify a Lease time-to-live of five seconds. This means the client will successfully send data to InfluxDB
for five seconds, and then begin receiving authentication errors. When the client receives an error, it will wait for
a keypress, and then request another lease. The keypress read is optional and only to provide you an opportunity to
inspect the status of your InfluxDB instance. It can be omitted.


## Client Source

Create `src/bin/client.rs` and add the following:

```rust
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
```

# Conclusion

This demo shows how Ockam can be used to securely manage the lifecycle of sensitive API tokens. Specifically, the Ockam
Lease Manager InfluxDB integration allows simple dynamic provisioning of API tokens with access control and lifecycle
management.

