use clap::crate_version;
use colorful::Colorful;
use serde::Deserialize;
use std::env;
use tokio::runtime::Builder;

#[derive(Deserialize)]
struct UpgradeFile {
    upgrade_to: String,
}

pub fn check_if_an_upgrade_is_available() {
    if !upgrade_check_is_disabled() {
        // check if a new version has been released
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(check());
    }
}

async fn check() {
    let url = format!(
        "https://github.com/build-trust/ockam/releases/download/ockam_v{}/upgrade.json",
        crate_version!()
    );
    let resp = reqwest::get(url).await;

    if let Ok(r) = resp {
        if let Ok(upgrade) = r.json::<UpgradeFile>().await {
            let new_version = upgrade.upgrade_to;
            eprintln!(
                "\n{}",
                format!(
                    "A new release of ockam is available: {} → {}",
                    crate_version!(),
                    new_version
                )
                .yellow()
            );
            if cfg!(target_os = "macos") {
                eprintln!(
                    "{}",
                    "To upgrade, run: `brew update && brew upgrade ockam`".yellow()
                );
            }
            eprintln!(
                "{}",
                format!(
                    "https://github.com/build-trust/ockam/releases/tag/ockam_v{}\n",
                    new_version
                )
                .yellow()
            );
        }
    }
}

fn upgrade_check_is_disabled() -> bool {
    match env::var("OCKAM_DISABLE_UPGRADE_CHECK") {
        Ok(v) => {
            let disable = v.trim().to_lowercase();
            disable == "1" || disable == "true" || disable == "yes"
        }
        Err(_e) => false,
    }
}
