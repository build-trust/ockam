use crate::error::Result;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
use std::fs;
use std::process::Stdio;
use std::{path::PathBuf, process::Command};

use crate::util::github::download_install_file_sync;

/// Get the appropriate installer to upgrade and uninstall ockam.
pub fn get_installer() -> Result<Box<dyn Installer>> {
    if installed_with_brew()? {
        return Ok(Box::<BrewInstaller>::default());
    }
    Ok(Box::<ScriptInstaller>::default())
}

pub trait Installer {
    /// Uninstall ockam.
    fn uninstall(&self) -> miette::Result<()>;
    /// Upgrade ockam to the specified version.
    fn upgrade(&self, version: &str) -> miette::Result<()>;

    fn home(&self) -> PathBuf {
        let home = std::env::var_os("HOME");
        match home {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from("~/"),
        }
    }

    fn ockam_home(&self) -> Result<PathBuf> {
        // check to see if OCKAM_HOME is set
        let ockam_home = std::env::var("OCKAM_HOME");
        if let Ok(home) = ockam_home {
            let mut pathbuf = PathBuf::new();
            pathbuf.push(home);
            return Ok(pathbuf);
        }

        // check to see if it is using the default ockam home
        let mut home = self.home();
        home.push(".ockam");
        if home.exists() && home.is_dir() {
            return Ok(home);
        }
        Err(miette!("Failed to get ockam home").into())
    }
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

#[derive(Debug, Default)]
pub struct BrewInstaller;

impl Installer for BrewInstaller {
    /// Upgrade ockam using homebrew. The version specified is ignored as homebrew
    /// can only upgrade to the latest version.
    fn upgrade(&self, _version: &str) -> miette::Result<()> {
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
    /// Upgrade ockam to the specified version.
    fn upgrade(&self, version: &str) -> miette::Result<()> {
        self.backup_files()?;
        let result = self.upgrade_binary(version);
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

    fn binary_path(&self) -> Result<PathBuf> {
        let binary_path = std::env::current_exe().into_diagnostic()?;
        Ok(binary_path)
    }

    fn binary_name(&self) -> Result<String> {
        let binary_path = self.binary_path()?;
        let name = binary_path
            .file_name()
            .ok_or(miette!("Failed to get binary name"))?
            .to_str()
            .ok_or(miette!("Failed to get binary name"))?;
        Ok(String::from(name))
    }

    fn uninstall_binary(&self) -> miette::Result<()> {
        let ockam_home = self.ockam_home()?;
        self.remove_lines_from_env_files()?;
        std::fs::remove_dir_all(ockam_home)
            .into_diagnostic()
            .context("Failed to uninstall ockam")
    }

    fn upgrade_binary(&self, version: &str) -> miette::Result<()> {
        self.remove_lines_from_env_files()?;

        let install_script_path = self.backup_folder.join("install.sh");
        download_install_file_sync(&install_script_path)?;

        let install_script_path_str = install_script_path
            .to_str()
            .expect("Invalid install script path");

        let ockam_home = self.ockam_home()?;
        let ockam_home = ockam_home
            .to_str()
            .ok_or(miette!("Failed to get ockam home"))?;
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
            let path = self.home().join(file);
            if path.exists() {
                let backup_path = self.backup_folder.join(file);
                std::fs::copy(&path, &backup_path)
                    .into_diagnostic()
                    .context("Failed to backup file")?;
            }
        }

        // for the binary, just move it into the backup folder
        let backup_path = self.backup_folder.join(self.binary_name()?);

        std::fs::rename(self.binary_path()?, backup_path)
            .into_diagnostic()
            .context("Failed to backup binary")?;

        Ok(())
    }

    fn restore_files(&self) -> miette::Result<()> {
        // restore all copied files
        for file in self.env_files.iter() {
            let path = self.home().join(file);
            let backup_path = self.backup_folder.join(file);
            if backup_path.exists() {
                std::fs::copy(&backup_path, &path)
                    .into_diagnostic()
                    .context("Failed to restore file")?;
            }
        }
        let backup_binary_path = self.backup_folder.join(self.binary_name()?);
        if backup_binary_path.exists() {
            std::fs::rename(&backup_binary_path, self.binary_path()?)
                .into_diagnostic()
                .context("Failed to restore binary")?;
        }
        Ok(())
    }

    fn cleanup(&self) -> miette::Result<()> {
        std::fs::remove_dir_all(&self.backup_folder)
            .into_diagnostic()
            .context("Failed to cleanup backup folder")
    }

    fn remove_lines_from_env_files(&self) -> miette::Result<()> {
        for file in self.env_files.iter() {
            let path = self.home().join(file);
            let contents_result = std::fs::read_to_string(&path);
            if let Ok(contents) = contents_result {
                let lines = contents.lines();
                let mut new_contents = String::new();
                for line in lines {
                    if !(line.contains("OCKAM_HOME") || line.contains(".ockam")) {
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
}
