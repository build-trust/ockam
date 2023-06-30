use crate::error::Result;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
use std::fs;
use std::fs::Permissions;
use std::path::Path;
use std::process::Stdio;
use std::{path::PathBuf, process::Command};

use crate::util::github::{download_install_file_sync, download_latest_binary_sync};

/// Get the appropriate installer to upgrade and uninstall ockam.
pub fn get_installer() -> Result<Box<dyn Installer>> {
    if installed_with_brew()? {
        return Ok(Box::<BrewInstaller>::default());
    } else if installed_with_script()? {
        return Ok(Box::<ScriptInstaller>::default());
    }
    Ok(Box::<BinaryInstaller>::default())
}

pub trait Installer {
    /// Uninstall ockam.
    fn uninstall(&self) -> miette::Result<()>;
    /// Upgrade ockam to the latest version.
    fn upgrade(&self) -> miette::Result<()>;
}

fn installed_with_brew() -> Result<bool> {
    // First we check to see if the ockam binary matches the brew list path
    // We check the canonical file paths to ensure that all paths are absoulte
    // and the simlinks are resolved.
    // Any unexpected errors here should cancel the upgrade/uninstallation process
    // as we don't want to accidentally delete the wrong files.
    let brew_prefix = Command::new("brew").args(["--prefix", "ockam"]).output();
    match brew_prefix {
        Ok(output) => {
            if output.status.success() {
                let brew_prefix_str = String::from_utf8(output.stdout).into_diagnostic()?;
                let brew_ockam_path = PathBuf::from(brew_prefix_str.trim()).join("bin/ockam");
                let canonical_brew_ockam_path =
                    fs::canonicalize(brew_ockam_path).into_diagnostic()?;
                let current_exe = std::env::current_exe().into_diagnostic()?;
                let canonical_current_exe = fs::canonicalize(current_exe).into_diagnostic()?;
                if canonical_brew_ockam_path == canonical_current_exe {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Err(_) => Ok(false),
    }
}

fn installed_with_script() -> Result<bool> {
    if let Ok(home) = ockam_home() {
        let ockam_bin = home.join("bin/ockam");
        if ockam_bin.exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn home() -> PathBuf {
    let home = std::env::var_os("HOME");
    match home {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from("~/"),
    }
}

fn ockam_home() -> Result<PathBuf> {
    // check to see if OCKAM_HOME is set
    let ockam_home = std::env::var("OCKAM_HOME");
    if let Ok(home) = ockam_home {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(home);
        return Ok(pathbuf);
    }

    // check to see if it is using the default ockam home
    let mut home = home();
    home.push(".ockam");
    if home.exists() && home.is_dir() {
        return Ok(home);
    }
    Err(miette!("Failed to get ockam home").into())
}

fn remove_ockam_lines_from_files(files: &[String]) -> miette::Result<()> {
    let home = home();
    for file in files.iter() {
        let path = home.join(file);
        remove_okcam_lines_from_file(path)?;
    }
    Ok(())
}

fn remove_okcam_lines_from_file(file: PathBuf) -> miette::Result<()> {
    let contents_result = std::fs::read_to_string(&file);
    if let Ok(contents) = contents_result {
        let lines = contents.lines();
        let mut new_contents = String::new();
        for line in lines {
            if !(line.contains("OCKAM_HOME") || line.contains(".ockam")) {
                new_contents.push_str(line);
                new_contents.push('\n');
            }
        }
        std::fs::write(&file, new_contents)
            .into_diagnostic()
            .context("Failed to write file")?;
    }
    Ok(())
}

fn binary_path() -> Result<PathBuf> {
    let binary_path = std::env::current_exe().into_diagnostic()?;
    Ok(binary_path)
}

fn binary_name() -> Result<String> {
    let binary_path = binary_path()?;
    let name = binary_path
        .file_name()
        .ok_or(miette!("Failed to get binary name"))?
        .to_str()
        .ok_or(miette!("Failed to get binary name"))?;
    Ok(String::from(name))
}

fn backup_binary(backup_folder: &Path) -> Result<()> {
    let backup_path = backup_folder.join(binary_name()?);

    std::fs::rename(binary_path()?, backup_path)
        .into_diagnostic()
        .context("Failed to backup binary")?;

    Ok(())
}

fn restore_binary(backup_folder: &Path) -> Result<()> {
    let backup_path = backup_folder.join(binary_name()?);
    if backup_path.exists() {
        std::fs::rename(&backup_path, binary_path()?)
            .into_diagnostic()
            .context("Failed to restore binary")?;
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct BrewInstaller;

impl Installer for BrewInstaller {
    /// Upgrade ockam using homebrew
    fn upgrade(&self) -> miette::Result<()> {
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
            Err(miette!("Failed to upgrade ockam with homebrew"))
        }
    }

    fn uninstall(&self) -> miette::Result<()> {
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
}

#[derive(Debug)]
pub struct ScriptInstaller {
    backup_folder: PathBuf,
    env_files: Vec<String>,
}

impl Installer for ScriptInstaller {
    /// Upgrade ockam to the latest version.
    fn upgrade(&self) -> miette::Result<()> {
        self.backup_files()?;
        let result = self.upgrade_binary();
        if result.is_err() {
            self.restore_files()?;
        }
        self.cleanup()?;
        result
    }

    /// Uninstall ockam.
    fn uninstall(&self) -> miette::Result<()> {
        self.backup_files()?;
        let result = self.uninstall_binary();
        if result.is_err() {
            self.restore_files()?;
        }
        self.cleanup()?;
        result
    }
}

impl Default for ScriptInstaller {
    fn default() -> Self {
        let env_files = vec![
            String::from(".profile"),
            String::from(".bash_profile"),
            String::from(".bash_login"),
            String::from(".bashrc"),
            String::from(".zshenv"),
        ];
        let backup_path = PathBuf::from("/tmp/ockam_backup");
        Self::new(backup_path, env_files)
    }
}

impl ScriptInstaller {
    pub fn new(backup_path: PathBuf, env_files: Vec<String>) -> Self {
        Self {
            backup_folder: backup_path,
            env_files,
        }
    }

    fn uninstall_binary(&self) -> miette::Result<()> {
        let ockam_home = ockam_home()?;
        remove_ockam_lines_from_files(&self.env_files)?;
        std::fs::remove_dir_all(ockam_home)
            .into_diagnostic()
            .context("Failed to uninstall ockam")
    }

    fn upgrade_binary(&self) -> miette::Result<()> {
        remove_ockam_lines_from_files(&self.env_files)?;

        let install_script_path = self.backup_folder.join("install.sh");
        download_install_file_sync(&install_script_path)?;

        let install_script_path_str = install_script_path
            .to_str()
            .expect("Invalid install script path");

        let ockam_home = ockam_home()?;
        let ockam_home = ockam_home
            .to_str()
            .ok_or(miette!("Failed to get ockam home"))?;
        let result = Command::new("sh")
            .args([install_script_path_str, "-p", ockam_home])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .into_diagnostic()
            .context("Failed to upgrade ockam")?;

        if result.status.success() {
            Ok(())
        } else {
            Err(miette!("Failed to uninstall ockam"))
        }
    }

    fn backup_files(&self) -> miette::Result<()> {
        if !self.backup_folder.exists() {
            std::fs::create_dir_all(&self.backup_folder)
                .into_diagnostic()
                .context("Failed to create backup folder")?;
        }
        // Copy all environment files into the backup folder
        for file in self.env_files.iter() {
            let path = home().join(file);
            if path.exists() {
                let backup_path = self.backup_folder.join(file);
                std::fs::copy(&path, &backup_path)
                    .into_diagnostic()
                    .context("Failed to backup file")?;
            }
        }
        backup_binary(&self.backup_folder)?;
        Ok(())
    }

    fn restore_files(&self) -> miette::Result<()> {
        // restore all copied files
        for file in self.env_files.iter() {
            let path = home().join(file);
            let backup_path = self.backup_folder.join(file);
            if backup_path.exists() {
                std::fs::copy(&backup_path, &path)
                    .into_diagnostic()
                    .context("Failed to restore file")?;
            }
        }
        restore_binary(&self.backup_folder)?;
        Ok(())
    }

    fn cleanup(&self) -> miette::Result<()> {
        std::fs::remove_dir_all(&self.backup_folder)
            .into_diagnostic()
            .context("Failed to cleanup backup folder")
    }
}

#[derive(Debug)]
pub struct BinaryInstaller {
    backup_folder: PathBuf,
}

impl Installer for BinaryInstaller {
    /// Upgrade ockam to the latest version.
    fn upgrade(&self) -> miette::Result<()> {
        let binary_name = self.get_binary_file_name()?;
        let install_path = binary_path()?;

        if !self.backup_folder.exists() {
            std::fs::create_dir_all(&self.backup_folder)
                .into_diagnostic()
                .context("Failed to create backup folder")?;
        }

        let permissions = fs::metadata(&install_path).into_diagnostic()?.permissions();

        backup_binary(&self.backup_folder)?;
        let result = self.download_and_configure_binary(&binary_name, &install_path, permissions);
        if result.is_err() {
            restore_binary(&self.backup_folder)?;
        }

        result
    }

    /// Uninstall ockam.
    fn uninstall(&self) -> miette::Result<()> {
        let binary_path = binary_path()?;
        backup_binary(&self.backup_folder)?;
        let result = std::fs::remove_dir_all(binary_path)
            .into_diagnostic()
            .context("Failed to uninstall ockam");
        if result.is_err() {
            restore_binary(&self.backup_folder)?;
        }
        result
    }
}

impl Default for BinaryInstaller {
    fn default() -> Self {
        let backup_folder = PathBuf::from("/tmp/ockam_backup");
        Self::new(backup_folder)
    }
}

impl BinaryInstaller {
    pub fn new(backup_folder: PathBuf) -> Self {
        Self { backup_folder }
    }

    fn download_and_configure_binary(
        &self,
        binary_name: &str,
        install_path: &PathBuf,
        permissions: Permissions,
    ) -> miette::Result<()> {
        download_latest_binary_sync(binary_name, install_path)?;
        fs::set_permissions(install_path, permissions).into_diagnostic()?;

        Ok(())
    }

    fn get_binary_file_name(&self) -> Result<String> {
        if cfg!(target_os = "macos") {
            if cfg!(target_arch = "x86_64") {
                return Ok(String::from("ockam.x86_64-apple-darwin"));
            }
            if cfg!(target_arch = "aarch64") {
                return Ok(String::from("ockam.aarch64-apple-darwin"));
            }
            return Err(miette!("Unsupported architecture").into());
        }
        if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86_64") {
                return Ok(String::from("ockam.x86_64-unknown-linux-musl"));
            }
            if cfg!(target_arch = "aarch64") {
                return Ok(String::from("ockam.aarch64-unknown-linux-musl"));
            }
            if cfg!(target_arch = "armv7") {
                return Ok(String::from("ockam.armv7-unknown-linux-musleabihf"));
            }
            return Err(miette!("Unsupported architecture").into());
        }
        Err(miette!("Unsupported OS").into())
    }
}
