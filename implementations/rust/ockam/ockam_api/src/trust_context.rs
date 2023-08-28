use crate::cli_state::{CliState, ProjectConfigCompact, StateDirTrait, StateItemTrait};
use crate::cloud::project::Project;
use crate::config::cli::TrustContextConfig;
use miette::{IntoDiagnostic, WrapErr};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TrustContextConfigBuilder {
    pub cli_state: CliState,
    pub project_path: Option<PathBuf>,
    pub trust_context: Option<TrustContextConfig>,
    pub project: Option<String>,
    pub authority_identity: Option<String>,
    pub credential_name: Option<String>,
    pub use_default_trust_context: bool,
}

impl TrustContextConfigBuilder {
    pub fn new(cli_state: &CliState) -> Self {
        Self {
            cli_state: cli_state.clone(),
            project_path: None,
            trust_context: None,
            project: None,
            authority_identity: None,
            credential_name: None,
            use_default_trust_context: false,
        }
    }

    pub fn with_authority_identity(&mut self, authority_identity: Option<&String>) -> &mut Self {
        self.authority_identity = authority_identity.map(|s| s.to_string());
        self
    }

    pub fn with_credential_name(&mut self, credential_name: Option<&String>) -> &mut Self {
        self.credential_name = credential_name.map(|s| s.to_string());
        self
    }

    pub fn use_default_trust_context(&mut self, use_default_trust_context: bool) -> &mut Self {
        self.use_default_trust_context = use_default_trust_context;
        self
    }

    pub fn build(&self) -> Option<TrustContextConfig> {
        self.trust_context
            .clone()
            .or_else(|| self.get_from_project_path(self.project_path.as_ref()?))
            .or_else(|| self.get_from_project_name())
            .or_else(|| self.get_from_authority_identity())
            .or_else(|| self.get_from_credential())
            .or_else(|| self.get_from_default_trust_context())
            .or_else(|| self.get_from_default_project())
    }

    fn get_from_project_path(&self, path: &PathBuf) -> Option<TrustContextConfig> {
        let s = std::fs::read_to_string(path)
            .into_diagnostic()
            .context("Failed to read project file")
            .ok()?;
        let proj_info = serde_json::from_str::<ProjectConfigCompact>(&s)
            .into_diagnostic()
            .context("Failed to parse project info")
            .ok()?;
        let proj: Project = (&proj_info).into();
        proj.try_into().ok()
    }

    fn get_from_project_name(&self) -> Option<TrustContextConfig> {
        let project = self.cli_state.projects.get(self.project.as_ref()?).ok()?;
        project.config().clone().try_into().ok()
    }

    fn get_from_authority_identity(&self) -> Option<TrustContextConfig> {
        let authority_identity = self.authority_identity.clone();
        let credential = match &self.credential_name {
            Some(c) => Some(self.cli_state.credentials.get(c).ok()?),
            None => None,
        };

        TrustContextConfig::from_authority_identity(&authority_identity?, credential).ok()
    }

    fn get_from_credential(&self) -> Option<TrustContextConfig> {
        let cred_name = self.credential_name.clone()?;
        let cred_state = self.cli_state.credentials.get(cred_name).ok()?;

        cred_state.try_into().ok()
    }

    fn get_from_default_trust_context(&self) -> Option<TrustContextConfig> {
        if !self.use_default_trust_context {
            return None;
        }

        let tc = self
            .cli_state
            .trust_contexts
            .default()
            .ok()?
            .config()
            .clone();
        Some(tc)
    }

    fn get_from_default_project(&self) -> Option<TrustContextConfig> {
        let proj = self.cli_state.projects.default().ok()?;
        self.get_from_project_path(proj.path())
    }
}
