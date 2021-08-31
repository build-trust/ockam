//! Utility that sleeps until a crate of a specific version is published on crates.io.
//! Usage: wait_for_crate cratename 0.1.0
use crates_io_api::SyncClient;
use std::time::Duration;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).take(2).collect();
    let crate_version = args.pop().expect("missing crate version");
    let crate_name = args.pop().expect("missing crate name");
    let client = SyncClient::new(
        "ockam_wait_for_crate (jared@ockam.io)",
        Duration::from_secs(1),
    )
    .expect("Couldn't create crates.io client");

    let mut found = false;
    while !found {
        if let Ok(info) = client.get_crate(crate_name.as_str()) {
            found = info.versions.iter().any(|v| v.num == crate_version);
        }
        std::thread::sleep(Duration::from_secs(1))
    }
}
