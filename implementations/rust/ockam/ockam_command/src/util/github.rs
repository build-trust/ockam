use std::{fs::File, io::Write, path::PathBuf};

use clap::crate_version;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use serde::Deserialize;
use tokio::runtime::Builder;

use crate::error::Result;

#[derive(Deserialize, Debug)]
pub struct LatestRelease {
    name: String,
}

impl LatestRelease {
    pub fn version(&self) -> Result<&str> {
        let result = self.name.split_once('v');
        match result {
            Some((_, version)) => Ok(version),
            None => Err(miette!("Failed to get latest release version").into()),
        }
    }
}

pub fn get_latest_release_version_sync() -> Result<LatestRelease> {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(get_latest_release_version())
}

pub async fn get_latest_release_version() -> Result<LatestRelease> {
    let url = "https://api.github.com/repos/build-trust/ockam/releases/latest";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Ockam Command")
        .send()
        .await;
    if let Ok(r) = resp {
        if let Ok(release) = r.json::<LatestRelease>().await {
            return Ok(release);
        }
    }
    Err(miette!("Failed to get latest release").into())
}

pub fn download_install_file_sync(install_path: &PathBuf) -> miette::Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(download_install_file(install_path))
}

pub async fn download_install_file(install_path: &PathBuf) -> miette::Result<()> {
    let install_script_url =
        "https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh";

    let client = reqwest::Client::new();
    let response = client
        .get(install_script_url)
        .send()
        .await
        .into_diagnostic()?;
    let mut file = File::create(install_path).into_diagnostic()?;
    file.write_all(&response.bytes().await.into_diagnostic()?)
        .into_diagnostic()
}

#[derive(Deserialize, Debug)]
struct UpgradeFile {
    upgrade_message: Option<String>,
    upgrade_message_macos: Option<String>,
}

pub fn check_upgrade_sync() {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(check_upgrade());
}

pub async fn check_upgrade() {
    let url = format!(
        "https://github.com/build-trust/ockam/releases/download/ockam_v{}/upgrade.json",
        crate_version!()
    );
    let resp = reqwest::get(url).await;

    if let Ok(r) = resp {
        if let Ok(upgrade) = r.json::<UpgradeFile>().await {
            if let Some(message) = upgrade.upgrade_message {
                eprintln!("\n{}", message.yellow());

                if cfg!(target_os = "macos") {
                    if let Some(message) = upgrade.upgrade_message_macos {
                        eprintln!("\n{}", message.yellow());
                    }
                }

                eprintln!();
            }
        }
    }
}

pub async fn download_latest_binary(name: &str, download_path: &PathBuf) -> miette::Result<()> {
    let url = format!(
        "https://github.com/build-trust/ockam/releases/latest/download/{}",
        name
    );
    let resp = reqwest::get(url).await.into_diagnostic()?;
    let bytes = resp.bytes().await.into_diagnostic()?;
    let mut file = File::create(download_path).into_diagnostic()?;
    file.write_all(&bytes).into_diagnostic()
}

pub fn download_latest_binary_sync(name: &str, download_path: &PathBuf) -> miette::Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(download_latest_binary(name, download_path))
}
