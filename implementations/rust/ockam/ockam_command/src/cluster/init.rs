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
use std::process::{Command as ProcessCommand, Stdio};

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
    /// It can be either a GitHub repository URL like `build-trust/ockam-cluster-template-hello`
    /// or an Ockam repository name that exists at `build-trust/ockam-cluster-template-<NAME>`
    repository_name: String,

    /// The path to install the template project.
    target_path: PathBuf,
}

impl InitCommand {
    fn get_github_url(&self) -> String {
        if self.repository_name.contains('/') {
            // URL for an arbitrary GitHub repository
            format!("https://github.com/{}.git", self.repository_name)
        } else {
            // URL for an ockam template
            format!(
                "https://github.com/build-trust/ockam-cluster-template-{}.git",
                self.repository_name
            )
        }
    }
}

#[async_trait]
impl Command for InitCommand {
    const NAME: &'static str = "cluster init";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let github_url = self.get_github_url();
        let temp_dir = tempfile::tempdir()
            .into_diagnostic()
            .wrap_err("Failed to create temporary directory")?;

        // Create target directory if it doesn't exist
        if !self.target_path.exists() {
            fs::create_dir_all(&self.target_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to create directory at {:?}", self.target_path)
                })?;
        } else if self
            .target_path
            .read_dir()
            .into_diagnostic()?
            .next()
            .is_some()
        {
            return Err(miette!(
                "Target directory {:?} already exists and is not empty",
                self.target_path
            ));
        }

        let spinner = opts.terminal.spinner();

        // Clone the repository into the temporary directory
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Downloading template from {}...",
                color_primary(&github_url)
            ));
        }

        let clone_status = tokio::process::Command::new("git")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .args(["clone", &github_url, "--depth", "1"])
            .current_dir(temp_dir.path())
            .status()
            .await
            .into_diagnostic()
            .wrap_err("Failed to execute git clone command")?;

        if !clone_status.success() {
            return Err(miette!("Failed to clone repository. Please check if the repository exists and you have internet access."));
        }

        // Get the repository directory name (last part of the URL before .git)
        let repo_dir_name = github_url
            .trim_end_matches(".git")
            .split('/')
            .last()
            .ok_or_else(|| miette!("Failed to parse repository name from URL"))?;

        let source_dir = temp_dir.path().join(repo_dir_name);

        // Copy all files except .git directory to the target path
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Copying template at {}...",
                color_primary(self.target_path.display())
            ));
        }

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

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        opts.terminal
            .to_stdout()
            .plain(fmt_ok!(
                "Successfully initialized template at {}",
                color_primary(self.target_path.display())
            ))
            .write_line()?;

        Ok(())
    }
}

// Recursively copy directories
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
