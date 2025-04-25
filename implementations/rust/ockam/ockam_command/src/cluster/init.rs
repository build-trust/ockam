use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_node::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tempfile::TempDir;
use url::Url;

const LONG_ABOUT: &str = include_str!("./static/init/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/init/after_long_help.txt");

/// Download and initialize a template project for an Ockam AI Agent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct InitCommand {
    /// The name of the template project to download.
    /// It can be either a GitHub repository like `build-trust/ockam-cluster-template-hello`,
    /// a full URL like `git@github.com:build-trust/ockam-cluster-template-hello`,
    /// or an Ockam repository name that exists at `build-trust/ockam-cluster-template-<NAME>`
    #[arg(default_value = "hello")]
    pub(crate) repository: String,

    /// The path to install the template project. Defaults to the current directory.
    pub(crate) target_path: Option<PathBuf>,
}

#[async_trait]
impl Command for InitCommand {
    const NAME: &'static str = "cluster init";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let spinner = opts.terminal.spinner();

        // Check if the current directory can be used
        let target_path = self.create_target_path()?;

        // Clone repository in a temporary directory
        let repository_url = self.get_repository_url();
        let temp_dir = tempfile::tempdir()
            .into_diagnostic()
            .wrap_err("Failed to create temporary directory")?;
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Downloading template from {}...",
                color_primary(&repository_url)
            ));
        }
        self.clone_repository(&repository_url, &temp_dir).await?;

        // Get the repository directory name (last part of the URL before .git)
        let repo_dir_name = repository_url
            .trim_end_matches(".git")
            .split('/')
            .last()
            .ok_or_else(|| miette!("Failed to parse repository name from URL"))?;

        // Copy all files except .git directory to the target path
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Copying template at {}...",
                color_primary(target_path.display())
            ));
        }
        self.copy_repository_files_to_target_path(&temp_dir, repo_dir_name, &target_path)
            .await?;

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        opts.terminal
            .to_stdout()
            .plain(fmt_ok!(
                "Successfully initialized template at {}",
                color_primary(target_path.display())
            ))
            .write_line()?;

        Ok(())
    }
}

impl InitCommand {
    fn get_repository_url(&self) -> String {
        let repository = self.repository.trim();
        if repository.ends_with(".git") {
            let repository = repository.trim_end_matches(".git");
            if Url::parse(repository).is_ok() {
                // An arbitrary URL
                format!("{}.git", repository)
            } else if repository.starts_with("git@") {
                // SSH URL for an arbitrary git repository
                format!("{}.git", repository)
            } else if repository.contains('/') {
                // URL for an arbitrary GitHub repository
                format!("https://github.com/{}.git", repository)
            } else {
                // URL for an ockam template
                format!(
                    "https://github.com/build-trust/ockam-cluster-template-{}.git",
                    repository
                )
            }
        } else {
            if Url::parse(repository).is_ok() {
                // An arbitrary URL
                repository.to_string()
            } else if repository.contains('/') {
                // URL for an arbitrary GitHub repository
                format!(
                    "https://github.com/{}/archive/refs/heads/main.zip",
                    repository
                )
            } else {
                // URL for an ockam template
                format!(
                    "https://github.com/build-trust/ockam-cluster-template-{}/archive/refs/heads/main.zip",
                    repository
                )
            }
        }
    }

    fn create_target_path(&self) -> Result<PathBuf> {
        let target_path = match &self.target_path {
            None => std::env::current_dir()
                .into_diagnostic()
                .wrap_err("Failed to get current directory")?,
            Some(path) => path.clone(),
        };
        if !target_path.exists() {
            fs::create_dir_all(&target_path)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to create directory at {:?}", target_path))?;
        } else if target_path.read_dir().into_diagnostic()?.next().is_some() {
            return Err(
                miette!("Target directory {:?} is not empty", target_path).wrap_err(format!(
                    "Use the command from an empty directory or pass the path to an empty directory using {}",
                    color_primary("--target-path")
                )),
            );
        }
        Ok(target_path)
    }

    async fn clone_repository(&self, repository_url: &str, temp_dir: &TempDir) -> Result<()> {
        if repository_url.ends_with(".git") {
            // Clone using git
            let clone_status = tokio::process::Command::new("git")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .args(["clone", repository_url, "--depth", "1"])
                .current_dir(temp_dir.path())
                .status()
                .await
                .into_diagnostic()
                .wrap_err("Failed to execute git clone command")?;

            if !clone_status.success() {
                return Err(miette!("Failed to clone repository. Please check if the repository exists and you have internet access."));
            }
        } else if repository_url.ends_with(".zip") {
            // Download using curl
            let archive_path = temp_dir.path().join("repo.zip");
            let curl_status = tokio::process::Command::new("curl")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .args([
                    "--silent",
                    "--show-error",
                    "--location", // Follow redirects
                    "--fail",     // Fail on HTTP errors
                    "--output",
                    archive_path.to_str().unwrap(),
                    repository_url,
                ])
                .current_dir(temp_dir.path())
                .status()
                .await
                .into_diagnostic()
                .wrap_err("Failed to execute curl command")?;
            if !curl_status.success() {
                return Err(miette!("Failed to download repository. Please check if the URL is valid and you have internet access."));
            }

            // Unzip
            let extract_status = tokio::process::Command::new("unzip")
                .args([
                    "-q", // Quiet mode
                    archive_path.to_str().unwrap(),
                    "-d",
                    temp_dir.path().to_str().unwrap(),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .into_diagnostic()
                .wrap_err("Failed to execute unzip command. Please ensure 'unzip' is installed.")?;

            if !extract_status.success() {
                return Err(miette!("Failed to extract repository archive."));
            }
        } else {
            return Err(miette!("Unsupported repository URL format"));
        }

        Ok(())
    }

    async fn copy_repository_files_to_target_path(
        &self,
        temp_dir: &TempDir,
        repo_dir_name: &str,
        target_path: &Path,
    ) -> Result<()> {
        let source_dir = temp_dir.path().join(repo_dir_name);
        for entry in fs::read_dir(&source_dir)
            .into_diagnostic()
            .wrap_err("Failed to read template directory")?
        {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();
            let file_name = path.file_name().unwrap();

            // Skip .git directory
            if file_name == ".git" {
                continue;
            }

            let target = target_path.join(file_name);

            if path.is_dir() {
                copy_dir_all(&path, &target)
                    .into_diagnostic()
                    .wrap_err_with(|| {
                        format!("Failed to copy directory from {:?} to {:?}", path, target)
                    })?;
            } else {
                fs::copy(&path, &target)
                    .into_diagnostic()
                    .wrap_err_with(|| {
                        format!("Failed to copy file from {:?} to {:?}", path, target)
                    })?;
            }
        }
        Ok(())
    }
}

/// Recursively copy directories
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();

        let name = path.file_name().unwrap();
        let dst_path = dst.join(name);

        if ty.is_dir() {
            copy_dir_all(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }
    Ok(())
}
