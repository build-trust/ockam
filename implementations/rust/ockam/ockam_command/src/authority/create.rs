use std::fmt::{Display, Formatter};

use clap::Args;
use miette::{miette, IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};
use tokio::process::Child;

use ockam::identity::models::ChangeHistory;
use ockam::identity::utils::now;
use ockam::identity::{Identifier, Identity, TimestampInSeconds, Vault};
use ockam::Context;
use ockam_api::authenticator::{PreTrustedIdentities, PreTrustedIdentity};
use ockam_api::authority_node;
use ockam_api::authority_node::{Authority, OktaConfiguration};
use ockam_api::colors::color_primary;
use ockam_api::config::lookup::InternetAddress;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::fmt;

use crate::node::util::run_ockam;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::foreground_args::{wait_for_exit_signal, ForegroundArgs};
use crate::util::parsers::internet_address_parser;
use crate::util::{async_cmd, local_cmd};
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create an Authority node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    /// Run the node in foreground.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    pub foreground: bool,

    /// Skip the check if such node is already running.
    /// Useful for kubernetes when the pid is the same on each run.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    skip_is_running_check: bool,

    /// `authority create` started a child process to run this node in foreground.
    #[arg(long, hide = true)]
    pub child_process: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:4000",
        value_parser = internet_address_parser
    )]
    tcp_listener_address: InternetAddress,

    /// Name of the Identity that the authority will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,

    /// Identifier of the project associated to this authority node on the Orchestrator
    #[arg(long, value_name = "PROJECT_IDENTIFIER")]
    project_identifier: String,

    /// Path to a file containing the identifier of the identity
    /// used by the project
    #[arg(long, value_name = "PROJECT_IDENTITY_IDENTIFIER_FILE")]
    project_identity_identifier_file: Option<String>,

    /// MultiAddr for accessing the project. If provided, then default project data is stored in the authority
    /// node database
    #[arg(long, value_name = "MULTI_ADDR")]
    project_access_route: Option<String>,

    /// List of the trusted identities, and corresponding attributes to be preload in the attributes storage.
    /// Format: {"identifier1": {"attribute1": "value1", "attribute2": "value12"}, ...}
    #[arg(long, value_name = "JSON_OBJECT", value_parser = parse_trusted_identities)]
    trusted_identities: TrustedIdentities,

    /// Set this option if the authority node should not support the enrollment
    /// of new project members
    #[arg(long, value_name = "BOOL", default_value_t = false)]
    no_direct_authentication: bool,

    /// Set this option if the authority node should not support
    /// the issuing of enrollment tokens
    #[arg(long, default_value_t = false)]
    no_token_enrollment: bool,

    /// Okta: URL used for accessing the Okta API
    #[arg(long, value_name = "URL", default_value = None)]
    tenant_base_url: Option<String>,

    /// Okta: pem certificate used to access the Okta server
    #[arg(long, value_name = "STRING", default_value = None)]
    certificate: Option<String>,

    /// Okta: name of the attributes which can be retrieved from Okta
    #[arg(long, value_name = "ATTRIBUTE_NAMES", default_value = None)]
    attributes: Option<Vec<String>>,

    /// Full, hex-encoded Identity (change history) of the account authority to trust
    /// for account and project administrator credentials.
    #[arg(long, value_name = "ACCOUNT_AUTHORITY_CHANGE_HISTORY", default_value = None, value_parser = ChangeHistory::import_from_string
    )]
    account_authority: Option<ChangeHistory>,

    /// Enforce distinction between admins and enrollers
    #[arg(long, value_name = "ENFORCE_ADMIN_CHECKS", default_value_t = false)]
    enforce_admin_checks: bool,

    /// Not include trust context id and project id into the credential
    /// TODO: Set to true after old clients are updated
    #[arg(long, value_name = "DISABLE_TRUST_CONTEXT_ID", default_value_t = false)]
    disable_trust_context_id: bool,
}

impl CreateCommand {
    pub fn name(&self) -> String {
        "authority create".to_string()
    }

    pub(crate) async fn spawn_background_node(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<Child> {
        if !self.skip_is_running_check {
            self.guard_node_is_not_already_running(opts).await?;
        }
        // Create the authority identity if it has not been created before
        // If no name is specified on the command line, create a unique name based on the project identifier
        let identity_name = self.identity.clone().unwrap_or(self.authority_name());
        if opts.state.get_named_identity(&identity_name).await.is_err() {
            opts.state.create_identity_with_name(&identity_name).await?;
        };

        opts.state
            .create_node_with_optional_identity(&self.node_name(), &self.identity)
            .await?;

        // Construct the arguments list and re-execute the ockam
        // CLI in foreground mode to start the newly created node
        let mut args = vec![
            match opts.global_args.verbose {
                0 => "-vv".to_string(),
                v => format!("-{}", "v".repeat(v as usize)),
            },
            "authority".to_string(),
            "create".to_string(),
            "--foreground".to_string(),
            "--child-process".to_string(),
            "--tcp-listener-address".to_string(),
            self.tcp_listener_address.to_string(),
            "--project-identifier".to_string(),
            self.project_identifier.clone(),
            "--trusted-identities".to_string(),
            self.trusted_identities.to_string(),
        ];

        if let Some(project_access_route) = self.project_access_route.clone() {
            args.push("--project-access-route".to_string());
            args.push(project_access_route);
        }

        if let Some(project_identity_identifier_file) =
            self.project_identity_identifier_file.clone()
        {
            args.push("--project-identity-identifier-file".to_string());
            args.push(project_identity_identifier_file);
        }

        if self.skip_is_running_check {
            args.push("--skip-is-running-check".to_string());
        }

        if self.logging_to_file() || !opts.terminal.is_tty() {
            args.push("--no-color".to_string());
        }

        if self.no_direct_authentication {
            args.push("--no-direct-authentication".to_string());
        }

        if self.no_token_enrollment {
            args.push("--no-token-enrollment".to_string());
        }

        if let Some(tenant_base_url) = &self.tenant_base_url {
            args.push("--tenant-base-url".to_string());
            args.push(tenant_base_url.clone());
        }

        if let Some(certificate) = &self.certificate {
            args.push("--certificate".to_string());
            args.push(certificate.clone());
        }

        if let Some(attributes) = &self.attributes {
            attributes.iter().for_each(|attr| {
                args.push("--attributes".to_string());
                args.push(attr.clone());
            });
        }

        if let Some(identity) = &self.identity {
            args.push("--identity".to_string());
            args.push(identity.clone());
        }
        if let Some(acc_auth_identity) = &self.account_authority {
            args.push("--account-authority".to_string());
            args.push(acc_auth_identity.export_as_string().into_diagnostic()?);
        }
        if self.enforce_admin_checks {
            args.push("--enforce-admin-checks".to_string());
        }
        if self.disable_trust_context_id {
            args.push("--disable_trust_context_id".to_string());
        }
        args.push(self.node_name());

        run_ockam(args, opts.global_args.quiet).await
    }
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if self.foreground {
            // Create a new node in the foreground (i.e. in this OS process)
            local_cmd(embedded_node_that_is_not_stopped(
                opts.rt.clone(),
                |ctx| async move { self.start_authority_node(&ctx, opts).await },
            ))
        } else {
            // Create a new node running in the background (i.e. another, new OS process)
            async_cmd(&self.name(), opts.clone(), |_ctx| async move {
                self.create_background_node(opts).await
            })
        }
    }

    /// Return a source of pre trusted identities and their attributes
    /// This is either a file which is used as the backend of the AttributesStorage
    /// or an explicit list of identities passed on the command line
    pub(crate) fn trusted_identities(
        &self,
        now: TimestampInSeconds,
        authority_identifier: &Identifier,
    ) -> PreTrustedIdentities {
        self.trusted_identities
            .to_pretrusted_identities(now, authority_identifier)
    }

    pub fn logging_to_file(&self) -> bool {
        // Background nodes will spawn a foreground node in a child process.
        // In that case, the child process will log to files.
        if self.child_process {
            true
        }
        // The main process will log to stdout only if it's a foreground node.
        else {
            !self.foreground
        }
    }

    /// Given a Context start a node in a new OS process
    async fn create_background_node(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        // Spawn node in another, new process
        self.spawn_background_node(&opts).await?;
        Ok(())
    }

    /// Start an authority node:
    ///   - retrieve the node identity if the authority identity has been created before
    ///   - persist the node state
    ///   - start the node services
    async fn start_authority_node(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.skip_is_running_check {
            self.guard_node_is_not_already_running(&opts).await?;
        }

        let state = opts.state.clone();

        // Create the authority identity if it has not been created before
        let identity_name = self.identity.clone().unwrap_or(self.authority_name());
        if opts.state.get_named_identity(&identity_name).await.is_err() {
            opts.state.create_identity_with_name(&identity_name).await?;
        };

        let node = state
            .start_node_with_optional_values(&self.node_name(), &Some(identity_name), None)
            .await?;
        state
            .set_tcp_listener_address(&self.node_name(), &self.tcp_listener_address)
            .await?;
        state.set_as_authority_node(&self.node_name()).await?;

        let okta_configuration = match (&self.tenant_base_url, &self.certificate, &self.attributes)
        {
            (Some(tenant_base_url), Some(certificate), Some(attributes)) => {
                Some(OktaConfiguration {
                    address: DefaultAddress::OKTA_IDENTITY_PROVIDER.to_string(),
                    tenant_base_url: tenant_base_url.clone(),
                    certificate: certificate.clone(),
                    attributes: attributes.clone(),
                })
            }
            _ => None,
        };

        let now = now().into_diagnostic()?;
        let trusted_identities = self.trusted_identities(now, &node.clone().identifier());

        let account_authority = match &self.account_authority {
            Some(account_authority_change_history) => Some(
                Identity::import_from_string(
                    None,
                    &account_authority_change_history
                        .export_as_string()
                        .into_diagnostic()?,
                    Vault::create_verifying_vault(),
                )
                .await
                .map(|i| i.change_history().clone())
                .into_diagnostic()?,
            ),
            None => None,
        };

        let configuration = authority_node::Configuration {
            identifier: node.identifier(),
            database_configuration: opts.state.database_configuration()?,
            project_identifier: self.project_identifier.clone(),
            tcp_listener_address: self.tcp_listener_address.clone(),
            secure_channel_listener_name: None,
            authenticator_name: None,
            trusted_identities,
            no_direct_authentication: self.no_direct_authentication,
            no_token_enrollment: self.no_token_enrollment,
            okta: okta_configuration,
            account_authority,
            enforce_admin_checks: self.enforce_admin_checks,
            disable_trust_context_id: self.disable_trust_context_id,
        };

        // SQLite doesn't like when the same database is opened by multiple times
        let database = if state.database_ref().configuration == configuration.database_configuration
        {
            Some(state.database())
        } else {
            None
        };

        // create the authority identity
        // or retrieve it from disk if the node has already been started before
        // The trusted identities in the configuration are used to pre-populate an attribute storage
        // containing those identities and their attributes
        let authority = Authority::create(&configuration, database)
            .await
            .into_diagnostic()?;

        authority_node::start_node(ctx, &configuration, authority)
            .await
            .into_diagnostic()?;

        let foreground_args = ForegroundArgs {
            child_process: self.child_process,
            exit_on_eof: false,
            foreground: self.foreground,
        };
        wait_for_exit_signal(
            &foreground_args,
            &opts,
            "To exit and stop the Authority node, please press Ctrl+C\n",
        )
        .await?;

        // Clean up and exit
        let _ = opts.state.stop_node(&self.node_name()).await;
        Ok(())
    }

    pub async fn guard_node_is_not_already_running(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.child_process {
            if let Ok(node) = opts.state.get_node(&self.node_name()).await {
                if node.is_running() {
                    return Err(miette!(
                        "Node {} is already running",
                        color_primary(self.node_name())
                    ));
                }
            }
        }
        Ok(())
    }

    // The authority identity name is built with the project identifier to make sure that it is unique.
    fn authority_name(&self) -> String {
        format!("authority-{}", self.project_identifier)
    }

    // The name of the authority node is the name of the authority to make sure that there are
    // no 2 authority nodes with the same name.
    pub fn node_name(&self) -> String {
        self.authority_name()
    }
}

/// Return a list of trusted identities passed as a JSON string on the command line
fn parse_trusted_identities(values: &str) -> Result<TrustedIdentities> {
    serde_json::from_str::<TrustedIdentities>(values)
        .into_diagnostic()
        .wrap_err("Cannot parse the trusted identities")
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct TrustedIdentities(BTreeMap<Identifier, BTreeMap<String, String>>);

impl TrustedIdentities {
    /// Return a map from Identifier to AttributesEntry and:
    ///   - add the project identifier as an attribute
    ///   - use the authority identifier an the attributes issuer
    pub(crate) fn to_pretrusted_identities(
        &self,
        now: TimestampInSeconds,
        authority_identifier: &Identifier,
    ) -> PreTrustedIdentities {
        let mut map = BTreeMap::<Identifier, PreTrustedIdentity>::default();
        for (identifier, attrs) in &self.0 {
            let attrs = attrs
                .iter()
                .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
                .collect();
            map.insert(
                identifier.clone(),
                PreTrustedIdentity::new(attrs, now, None, authority_identifier.clone()),
            );
        }
        PreTrustedIdentities::new(map)
    }
}

impl Display for TrustedIdentities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| fmt::Error)?
                .as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use ockam::identity::{identities, Identifier};
    use ockam_api::authenticator::direct::{
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
    };

    use super::*;

    #[tokio::test]
    async fn test_parse_trusted_identities() -> Result<()> {
        let authority = create_identity().await?;
        let identifier1 = create_identity().await?;
        let identifier2 = create_identity().await?;

        let trusted = format!("{{\"{identifier1}\": {{\"name\": \"value\"}}, \"{identifier2}\": {{\"ockam-role\" : \"enroller\"}}}}");
        let actual = parse_trusted_identities(trusted.as_str()).unwrap();

        let now = now()?;
        let pre_trusted_identities = actual.to_pretrusted_identities(now, &authority);

        assert_eq!(pre_trusted_identities.len(), 2);

        let id1 = pre_trusted_identities.get(&identifier1).unwrap();
        assert_eq!(id1.attrs().len(), 1);
        assert_eq!(
            id1.attrs().get(&"name".as_bytes().to_vec()),
            Some(&"value".as_bytes().to_vec())
        );

        let id2 = pre_trusted_identities.get(&identifier2).unwrap();
        assert_eq!(id2.attrs().len(), 1);
        assert_eq!(
            id2.attrs()
                .get(&OCKAM_ROLE_ATTRIBUTE_KEY.as_bytes().to_vec()),
            Some(&OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes().to_vec())
        );

        Ok(())
    }

    /// HELPERS
    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        Ok(identities.identities_creation().create_identity().await?)
    }
}
