use crate::zone::common_args::ZoneConfigArg;
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
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct InitCommand {
    /// The name of the template project to download.
    /// It can be either a GitHub repository like `build-trust/ockam-cluster-template-hello`,
    /// a full URL like `git@github.com:build-trust/ockam-cluster-template-hello`,
    /// an Ockam repository name that exists at `build-trust/ockam-cluster-template-<NAME>`,
    /// an Ockam example like `build-trust/ockam/examples/001`,
    /// or a URL to a ZIP archive like `"https://github.com/build-trust/ockam-cluster-template-hello/archive/refs/heads/main.zip
    #[arg(default_value = "hello")]
    pub(crate) repository: String,

    /// The path to install the template project. Defaults to the current directory.
    pub(crate) target_path: Option<PathBuf>,
}

#[async_trait]
impl Command<Option<PathBuf>> for InitCommand {
    const NAME: &'static str = "zone init";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<Option<PathBuf>> {
        // Check if the target directory can be used
        let target_path = match self.create_target_path()? {
            Some(path) => path,
            None => {
                return Ok(None);
            }
        };
        let target_path_str = target_path.display().to_string();

        // Clone repository in a temporary directory
        let repository_downloader =
            RepositoryDownloader::new(&self.repository, target_path.clone())?;
        let repository_url = &repository_downloader.repository_url;
        let spinner = opts.terminal.spinner();
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

        Ok(Some(target_path))
    }
}

impl InitCommand {
    pub fn create_target_path(&self) -> Result<Option<PathBuf>> {
        let create_target_path = |path: &PathBuf| {
            if !path.exists() {
                fs::create_dir_all(path)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Failed to create directory at {:?}", path))?;
            }
            std::env::set_current_dir(path)
                .into_diagnostic()
                .wrap_err(format!("Failed to set current directory to {:?}", path))?;
            Ok::<_, miette::Error>(path.clone())
        };
        let zone_config_arg = ZoneConfigArg::default();
        let check_zone_config = |path: &PathBuf| -> Result<()> {
            if zone_config_arg.zone_config_path().is_ok() {
                if zone_config_arg.zone_config().is_err() {
                    Err(miette!(
                        "Target directory {:?} doesn't contain a valid zone config file",
                        path
                    )
                    .wrap_err(format!(
                        "Use the command from a valid directory or pass a directory using {}",
                        color_primary("--target-path")
                    )))
                } else {
                    Ok(())
                }
            } else {
                Err(miette!(
                    "No zone config file found in target directory {:?}",
                    path
                ))
            }
        };

        let target_path = match &self.target_path {
            None => std::env::current_dir()
                .into_diagnostic()
                .wrap_err("Failed to get current directory")?,
            Some(path) => path.clone(),
        };
        let current_dir = create_target_path(&target_path)?;

        if target_path.read_dir().into_diagnostic()?.next().is_some() {
            // Target is not empty

            // Check if it contains a valid zone config file
            if zone_config_arg.zone_config_path().is_ok() {
                // A zone config file exists
                check_zone_config(&target_path)?;
                Ok(None)
            } else {
                // No zone config file found
                let target_path = current_dir.join(self.repository_name());
                create_target_path(&target_path)?;
                if target_path.read_dir().into_diagnostic()?.next().is_some() {
                    check_zone_config(&target_path)?;
                    Ok(None)
                } else {
                    Ok(Some(target_path))
                }
            }
        } else {
            Ok(Some(target_path.clone()))
        }
    }

    fn repository_name(&self) -> String {
        let name = self
            .repository
            .trim_end_matches(".git")
            .trim_end_matches(".zip")
            .trim_end_matches("/archive/refs/heads/main")
            .trim_end_matches("/archive/refs/heads/master")
            .trim_end_matches("/archive/refs/heads/develop")
            .replace("ockam-cluster-template-", "");
        let name = name.split('/').next_back().unwrap_or(&name);
        name.to_string()
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
    repository_subdir: Option<PathBuf>,
    temp_dir: TempDir,
    target_path: PathBuf,
}

impl RepositoryDownloader {
    fn new(repository_hint: &str, target_path: PathBuf) -> Result<Self> {
        let mut repository_subdir = None;
        let mut repository_hint = repository_hint.trim();
        let repository_type = if repository_hint.ends_with(".git") {
            RepositoryType::Git
        } else if repository_hint.contains("build-trust/ockam/examples") {
            // If repository_hint == "build-trust/ockam/examples/001", then
            //  repository_subdir = Some(PathBuf::from("examples/001"))
            //  and repository_hint = "build-trust/ockam"
            repository_subdir = Some(PathBuf::from(
                repository_hint
                    .split("build-trust/ockam/")
                    .nth(1)
                    .ok_or(miette!("Failed to parse repository subdir"))?,
            ));
            repository_hint = "build-trust/ockam";
            RepositoryType::Zip
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
            repository_subdir,
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
                    let branch = if hint.contains("build-trust/ockam") {
                        "develop"
                    } else {
                        "main"
                    };
                    format!(
                        "https://github.com/{}/archive/refs/heads/{}.zip",
                        hint, branch
                    )
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
        let mut source_dir = match self.repository_type {
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
        // If a subdirectory is specified, use it as the source directory
        source_dir = if let Some(subdir) = &self.repository_subdir {
            source_dir.join(subdir)
        } else {
            source_dir
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_create_target_path_explicit_empty_dir() -> Result<()> {
        let temp_dir = TempDir::new().into_diagnostic()?;
        let target_dir = temp_dir.path().join("explicit-dir");
        fs::create_dir_all(&target_dir).into_diagnostic()?;

        let cmd = InitCommand {
            repository: "test-repo".to_string(),
            target_path: Some(target_dir.clone()),
        };

        let result = cmd.create_target_path()?;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), target_dir);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_create_target_path_non_empty_without_config_creates_subdir() -> Result<()> {
        let temp_dir = TempDir::new().into_diagnostic()?;
        std::env::set_current_dir(&temp_dir).into_diagnostic()?;

        File::create("some-file.txt").into_diagnostic()?;

        let repo_name = "test-repo";
        let cmd = InitCommand {
            repository: repo_name.to_string(),
            target_path: None,
        };

        let result = cmd.create_target_path()?;
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().file_name().unwrap().to_str().unwrap(),
            repo_name
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_create_target_path_non_empty_subdir_without_config() -> Result<()> {
        let temp_dir = TempDir::new().into_diagnostic()?;
        std::env::set_current_dir(&temp_dir).into_diagnostic()?;

        // Create a subdirectory with the repository name but no valid config
        let repo_name = "test-repo";
        let subdir = temp_dir.path().join(repo_name);
        fs::create_dir_all(&subdir).into_diagnostic()?;
        File::create(subdir.join("some-file.txt")).into_diagnostic()?;

        let cmd = InitCommand {
            repository: repo_name.to_string(),
            target_path: None,
        };

        // This should fail because both current dir and subdir are non-empty and without a config
        let result = cmd.create_target_path();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_repository_name_extraction() {
        let test_cases = [
            // Simple name
            ("hello", "hello"),
            // With .git extension
            ("hello.git", "hello"),
            // With .zip extension
            ("hello.zip", "hello"),
            // Full GitHub URL
            ("https://github.com/build-trust/ockam-cluster-template-hello", "hello"),
            // SSH URL
            ("git@github.com:build-trust/ockam-cluster-template-hello.git", "hello"),
            // URL with branch
            ("repo/archive/refs/heads/main", "repo"),
            ("repo/archive/refs/heads/master", "repo"),
            ("repo/archive/refs/heads/develop", "repo"),
            // Template name with prefix
            ("ockam-cluster-template-example", "example"),
            // GitHub path
            ("user/repo", "repo"),
            ("user/repo.git", "repo"),
            ("org/team/repo", "repo"),
            // Combined cases
            ("build-trust/ockam-cluster-template-hello.git", "hello"),
            ("https://github.com/build-trust/ockam-cluster-template-hello/archive/refs/heads/main", "hello"),
        ];

        for (input, expected) in test_cases {
            let cmd = InitCommand {
                repository: input.to_string(),
                target_path: None,
            };
            assert_eq!(
                cmd.repository_name(),
                expected,
                "Failed for input: {}",
                input
            );
        }
    }
}
