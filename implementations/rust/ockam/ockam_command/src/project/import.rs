use clap::ArgGroup;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam_api::cloud::project::models::ProjectModel;
use ockam_api::{fmt_err, fmt_ok};

use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/import/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/import/after_long_help.txt");

/// Import a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
#[clap(group(ArgGroup::new("detailed").required(false)))]
pub struct ImportCommand {
    /// Project file
    #[arg(long, value_name = "PATH")]
    pub project_file: String,
}

impl ImportCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "project import".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let file_content = std::fs::read_to_string(&self.project_file).into_diagnostic()?;
        let project: ProjectModel = serde_json::from_str(&file_content).into_diagnostic()?;
        let result = opts
            .state
            .projects()
            .import_and_store_project(project.clone())
            .await;

        match result {
            Ok(_) => opts
                .terminal
                .stdout()
                .plain(fmt_ok!("Successfully imported project {}", &project.name))
                .write_line()?,
            Err(e) => opts
                .terminal
                .stdout()
                .plain(fmt_err!(
                    "The project {} could not be imported: {}",
                    &self.project_file,
                    e.to_string()
                ))
                .write_line()?,
        };
        Ok(())
    }
}
