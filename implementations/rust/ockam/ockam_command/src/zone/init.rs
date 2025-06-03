use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_node::Context;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tempfile::TempDir;
use tracing::{debug, info};
use url::Url;

const LONG_ABOUT: &str = include_str!("./static/init/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/init/after_long_help.txt");

/// Download and initialize a template project
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
    const NAME: &'static str = "zone init";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let spinner = opts.terminal.spinner();

        // Check if the current directory can be used
        let target_path = self.create_target_path()?;
        let target_path_str = target_path.display().to_string();

        // Clone repository in a temporary directory
        let repository_downloader = RepositoryDownloader::new(&self.repository, target_path)?;
        let repository_url = &repository_downloader.repository_url;
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Downloading template from {}...",
                color_primary(repository_url)
            ));
        }
        repository_downloader.clone_repository().await?;

        // Copy all files except .git directory to the target path
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Copying template at {}...",
                color_primary(&target_path_str)
            ));
        }
        repository_downloader
            .copy_repository_files_to_target_path()
            .await?;

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        opts.terminal
            .to_stdout()
            .plain(fmt_ok!(
                "Initialized template at {}",
                color_primary(&target_path_str)
            ))
            .write_line()?;

        Ok(())
    }
}

impl InitCommand {
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

struct RepositoryDownloader {
    repository_url: String,
    repository_type: RepositoryType,
    temp_dir: TempDir,
    target_path: PathBuf,
}

impl RepositoryDownloader {
    fn new(repository_hint: &str, target_path: PathBuf) -> Result<Self> {
        let repository_type = if repository_hint.ends_with(".git") {
            RepositoryType::Git
        } else {
            // Defaults to ZIP when a repository name or a template name is passed
            RepositoryType::Zip
        };
        let temp_dir = tempfile::tempdir()
            .into_diagnostic()
            .wrap_err("Failed to create temporary directory")?;
        Ok(Self {
            repository_url: Self::_build_repository_url_from_hint(
                repository_hint,
                &repository_type,
            ),
            repository_type,
            temp_dir,
            target_path,
        })
    }

    fn _build_repository_url_from_hint(hint: &str, repository_type: &RepositoryType) -> String {
        match repository_type {
            RepositoryType::Git => {
                let hint = hint.trim_end_matches(".git");
                if Url::parse(hint).is_ok() {
                    // An arbitrary URL
                    format!("{}.git", hint)
                } else if hint.starts_with("git@") {
                    // SSH URL for a git repository
                    format!("{}.git", hint)
                } else if hint.contains('/') {
                    // A "<user>/<repo>" string
                    format!("https://github.com/{}.git", hint)
                } else {
                    // An Ockam template name
                    format!(
                        "https://github.com/build-trust/ockam-cluster-template-{}.git",
                        hint
                    )
                }
            }
            RepositoryType::Zip => {
                if Url::parse(hint).is_ok() {
                    // An arbitrary URL
                    hint.to_string()
                } else if hint.contains('/') {
                    // A "<user>/<repo>" string
                    format!("https://github.com/{}/archive/refs/heads/main.zip", hint)
                } else {
                    // An Ockam template name
                    format!(
                        "https://github.com/build-trust/ockam-cluster-template-{}/archive/refs/heads/main.zip",
                        hint
                    )
                }
            }
        }
    }

    async fn clone_repository(&self) -> Result<()> {
        match self.repository_type {
            RepositoryType::Git => self._clone_git_repository().await,
            RepositoryType::Zip => self._download_zip_repository().await,
        }
    }

    async fn _clone_git_repository(&self) -> Result<()> {
        debug!("Cloning git repository from {}", self.repository_url);
        let clone_status = tokio::process::Command::new("git")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .args(["clone", &self.repository_url, "--depth", "1"])
            .current_dir(self.temp_dir.path())
            .status()
            .await
            .into_diagnostic()
            .wrap_err("Failed to execute git clone command")?;
        if !clone_status.success() {
            Err(miette!("Failed to clone repository. Please check if the repository exists and you have internet access."))
        } else {
            info!("Cloned git repository from {}", self.repository_url);
            Ok(())
        }
    }

    async fn _download_zip_repository(&self) -> Result<()> {
        // Download zip
        debug!("Downloading zip repository from {}", self.repository_url);
        let archive_path = self.temp_dir.path().join("repo.zip");
        let response = reqwest::get(&self.repository_url)
            .await
            .into_diagnostic()
            .wrap_err("Failed to download repository")?;
        if !response.status().is_success() {
            return Err(miette!(
                "Failed to download repository. Server returned status: {}",
                response.status()
            ));
        }
        let bytes = response
            .bytes()
            .await
            .into_diagnostic()
            .wrap_err("Failed to read response body")?;
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .into_diagnostic()
            .wrap_err("Failed to create archive file")?;
        tokio::io::copy(&mut bytes.as_ref(), &mut file)
            .await
            .into_diagnostic()
            .wrap_err("Failed to write archive file")?;
        info!("Downloaded zip repository to {}", archive_path.display());

        // Unzip
        debug!(
            "Extracting zip repository to {}",
            self.temp_dir.path().display()
        );
        let _temp_dir = self.temp_dir.path().to_path_buf();
        let _archive_path = archive_path.clone();
        tokio::task::spawn_blocking(move || {
            zip_extract::extract(
                fs::File::open(&_archive_path)
                    .into_diagnostic()
                    .wrap_err("Failed to open downloaded archive")?,
                &_temp_dir,
                true,
            )
            .into_diagnostic()
            .wrap_err("Failed to extract ZIP archive")?;
            Ok::<_, miette::Error>(())
        })
        .await
        .into_diagnostic()
        .wrap_err("Failed to complete ZIP extraction")??;

        // Remove the archive after extraction
        tokio::fs::remove_file(&archive_path)
            .await
            .into_diagnostic()?;
        info!(
            "Extracted zip repository to {}",
            self.temp_dir.path().display()
        );

        Ok(())
    }

    async fn copy_repository_files_to_target_path(&self) -> Result<()> {
        let source_dir = match self.repository_type {
            RepositoryType::Git => {
                let repo_name = self
                    .repository_url
                    .trim_end_matches(".git")
                    .split('/')
                    .next_back()
                    .ok_or_else(|| miette!("Failed to parse repository name from URL"))?;
                self.temp_dir.path().join(repo_name)
            }
            RepositoryType::Zip => self.temp_dir.path().to_path_buf(),
        };
        debug!(
            "Copying repository files to target path {:?} from {:?}",
            self.target_path, source_dir
        );
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

            let target = self.target_path.join(file_name);

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
        info!(
            "Copied repository files to target path {:?} from {:?}",
            self.target_path, source_dir
        );
        Ok(())
    }
}

enum RepositoryType {
    Git,
    Zip,
}
