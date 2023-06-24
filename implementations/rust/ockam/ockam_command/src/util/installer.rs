use crate::error::Result;
use clap::crate_version;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
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

fn upgrade_binary(version: &str) -> miette::Result<()> {
    let ockam_home = get_ockam_home()?;
    let ockam_home = ockam_home.to_str().expect("Invalid ockam home");
    delete_binary()?;

    let result = Command::new("sh")
        .args([
            &format!("{}/install.sh", ockam_home),
            "-p",
            ockam_home,
            "-v",
            &format!("v{}", version),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .into_diagnostic()
        .context("Failed to upgrade ockam")?;

    if result.status.success() {
        Ok(())
    } else {
        Err(miette!("Failed to upgrade ockam"))
    }
}

fn upgrade_check() -> miette::Result<()> {
    // Check to see if everything is okay for us to update the binary.
    let mut ockam_home = get_ockam_home()?;
    ockam_home.push("install.sh");
    if ockam_home.exists() && ockam_home.is_file() {
        return Ok(());
    }
    Err(miette!("Failed to find install.sh, unable to run upgrade"))
}

/// Upgrade ockam to the specified version.
/// If ockam was installed with brew, then upgrade with brew.
/// Otherwise, upgrade with the install script.
pub fn upgrade(version: &str) -> miette::Result<()> {
    let current_version = crate_version!();
    if check_installed_with_brew(current_version) {
        return upgrade_with_brew();
    }
    upgrade_check()?;
    upgrade_binary(version)
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
                if !line.contains("OCKAM_HOME") {
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
