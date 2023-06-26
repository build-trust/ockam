use crate::error::Result;
use clap::crate_version;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
use std::fs::File;
use std::io::Write;
use std::process::Stdio;
use std::{path::PathBuf, process::Command};

fn upgrade_with_brew() -> miette::Result<()> {
    let result = Command::new("brew")
        .args(["upgrade", "ockam"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .into_diagnostic()
        .context("Failed to upgrade ockam with homebrew")?;
    if result.status.success() {
        Ok(())
    } else {
        Err(miette!("Failed to upgrade ockam here"))
    }
}

fn check_installed_with_brew(current_version: &str) -> bool {
    // if brew finds ockam and it is the same version as the current version, then
    // ockam has been installed with brew and so we upgrade with brew
    let brew_result = Command::new("brew")
        .args(["ls", "--versions", "ockam"])
        .output();
    if let Ok(output) = brew_result {
        if !output.stdout.is_empty() {
            let brew_version_result = String::from_utf8(output.stdout);
            if let Ok(brew_version) = brew_version_result {
                if brew_version.trim().contains(current_version) {
                    return true;
                }
            }
        }
    }
    false
}

fn uninstall_with_brew() -> miette::Result<()> {
    let result = Command::new("brew")
        .args(["uninstall", "ockam"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .into_diagnostic()
        .context("Failed to uninstall ockam with homebrew")?;
    if result.status.success() {
        Ok(())
    } else {
        Err(miette!("Failed to uninstall ockam with homebrew"))
    }
}

fn get_ockam_home() -> Result<PathBuf> {
    // check to see if OCKAM_HOME is set
    let ockam_home = std::env::var("OCKAM_HOME");
    if let Ok(home) = ockam_home {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(home);
        return Ok(pathbuf);
    }

    // check to see if it is using the default ockam home
    let home = std::env::var("HOME");
    if let Ok(home) = home {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(home);
        pathbuf.push(".ockam");
        if pathbuf.exists() && pathbuf.is_dir() {
            return Ok(pathbuf);
        }
    }
    Err(miette!("Failed to get ockam home").into())
}

fn delete_binary() -> miette::Result<()> {
    let binary_path = std::env::current_exe().into_diagnostic()?;
    std::fs::remove_file(binary_path)
        .into_diagnostic()
        .context("Failed to delete binary")
}

async fn upgrade_binary(version: &str) -> miette::Result<()> {
    let ockam_home = get_ockam_home()?;
    let ockam_home = ockam_home.to_str().expect("Invalid ockam home");

    let install_script_path = download_install_file().await?;
    let install_script_path_str = install_script_path
        .to_str()
        .expect("Invalid install script path");

    let binary_path = std::env::current_exe().into_diagnostic()?;

    let mut backup_path = PathBuf::from("/tmp");
    backup_path.push("ockam.bak");

    std::fs::rename(&binary_path, &backup_path)
        .into_diagnostic()
        .context("Failed to backup binary")?;

    let result = Command::new("sh")
        .args([
            install_script_path_str,
            "-p",
            ockam_home,
            "-v",
            &format!("v{}", version),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output();

    let _ = std::fs::remove_file(&install_script_path);

    match result {
        Ok(output) => {
            if output.status.success() {
                std::fs::remove_file(&backup_path)
                    .into_diagnostic()
                    .context("Failed to delete backup binary")
            } else {
                std::fs::rename(&backup_path, &binary_path)
                    .into_diagnostic()
                    .context("Failed to restore binary")?;
                Err(miette!("Failed to upgrade ockam"))
            }
        }
        Err(_) => {
            std::fs::rename(&backup_path, &binary_path)
                .into_diagnostic()
                .context("Failed to restore binary")?;
            Err(miette!("Failed to upgrade ockam"))
        }
    }
}

async fn download_install_file() -> Result<PathBuf> {
    let install_script_url =
        "https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh";
    let mut install_path = PathBuf::from("/tmp");
    install_path.push("install.sh");

    let client = reqwest::Client::new();
    let response = client
        .get(install_script_url)
        .send()
        .await
        .into_diagnostic()?;
    let mut file = File::create(&install_path)?;
    file.write_all(&response.bytes().await.into_diagnostic()?)?;

    Ok(install_path)
}

/// Upgrade ockam to the specified version.
/// If ockam was installed with brew, then upgrade with brew.
/// Otherwise, upgrade with the install script.
pub async fn upgrade(version: &str) -> miette::Result<()> {
    let current_version = crate_version!();
    if check_installed_with_brew(current_version) {
        return upgrade_with_brew();
    }
    upgrade_binary(version).await
}

fn remove_lines_from_env_files() -> miette::Result<()> {
    let files = vec![
        ".profile",
        ".bash_profile",
        ".bash_login",
        ".bashrc",
        ".zshenv",
    ];
    for file in files {
        let home = std::env::var("HOME").expect("Failed to get home dir");
        let path = format!("{}/{}", home, file);
        let contents_result = std::fs::read_to_string(&path);
        if let Ok(contents) = contents_result {
            let lines = contents.lines();
            let mut new_contents = String::new();
            for line in lines {
                if !line.contains("OCKAM_HOME") || !line.contains(".ockam") {
                    new_contents.push_str(line);
                    new_contents.push('\n');
                }
            }
            std::fs::write(&path, new_contents)
                .into_diagnostic()
                .context("Failed to write file")?;
        }
    }
    Ok(())
}

/// Uninstall ockam.
/// If ockam was installed with brew, then uninstall with brew.
/// Otherwise delete ockam binary.
/// Then delete $OCKAM_HOME
pub fn uninstall() -> miette::Result<()> {
    let current_version = crate_version!();
    if check_installed_with_brew(current_version) {
        uninstall_with_brew()?;
    } else {
        delete_binary()?;
    }

    let ockam_home = get_ockam_home()?;
    remove_lines_from_env_files()?;
    std::fs::remove_dir_all(ockam_home)
        .into_diagnostic()
        .context("Failed to uninstall ockam")
}
