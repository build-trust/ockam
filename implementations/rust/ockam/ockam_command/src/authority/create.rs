use crate::node::util::run_ockam;
use crate::util::{embedded_node_that_is_not_stopped, exitcode};
use crate::util::{local_cmd, node_rpc};
use crate::{docs, identity, CommandGlobalOpts, Result};
use clap::{ArgGroup, Args};
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use ockam::identity::{AttributesEntry, Identifier};
use ockam::Context;
use ockam_api::authority_node;
use ockam_api::authority_node::{OktaConfiguration, TrustedIdentity};
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_api::cli_state::init_node_state;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use ockam_api::DefaultAddress;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::fmt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use tracing::debug;

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
#[clap(group(ArgGroup::new("trusted").required(true).args(& ["trusted_identities", "reload_from_trusted_identities_file"])))]
pub struct CreateCommand {
    /// Name of the node
    #[arg(default_value = "authority")]
    node_name: String,

    /// Identifier of the project associated to this authority node on the Orchestrator
    #[arg(long, value_name = "PROJECT_IDENTIFIER")]
    project_identifier: String,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:4000"
    )]
    tcp_listener_address: String,

    /// `authority create` started a child process to run this node in foreground.
    #[arg(long, hide = true)]
    pub child_process: bool,

    /// Set this option if the authority node should not support the enrollment
    /// of new project members
    #[arg(long, value_name = "BOOL", default_value_t = false)]
    no_direct_authentication: bool,

    /// Set this option if the authority node should not support
    /// the issuing of enrollment tokens
    #[arg(long, default_value_t = false)]
    no_token_enrollment: bool,

    /// List of the trusted identities, and corresponding attributes to be preload in the attributes storage.
    /// Format: {"identifier1": {"attribute1": "value1", "attribute2": "value12"}, ...}
    #[arg(group = "trusted", long, value_name = "JSON_OBJECT", value_parser = parse_trusted_identities)]
    trusted_identities: Option<TrustedIdentities>,

    /// Path of a file containing trusted identities and their attributes encoded as a JSON object.
    /// Format: {"identifier1": {"attribute1": "value1", "attribute2": "value12"}, ...}
    #[arg(group = "trusted", long, value_name = "PATH")]
    reload_from_trusted_identities_file: Option<PathBuf>,

    /// Okta: URL used for accessing the Okta API
    #[arg(long, value_name = "URL", default_value = None)]
    tenant_base_url: Option<String>,

    /// Okta: pem certificate used to access the Okta server
    #[arg(long, value_name = "STRING", default_value = None)]
    certificate: Option<String>,

    /// Okta: name of the attributes which can be retrieved from Okta
    #[arg(long, value_name = "ATTRIBUTE_NAMES", default_value = None)]
    attributes: Option<Vec<String>>,

    /// Run the node in foreground.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    foreground: bool,

    /// Vault that authority will use
    #[arg(long = "vault", value_name = "VAULT_NAME")]
    vault: Option<String>,

    /// Name of the Identity that the authority will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,
}

/// Start an authority node by calling the `ockam` executable with the current command-line
/// arguments
async fn spawn_background_node(
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
) -> miette::Result<()> {
    // Create node state, including the vault and identity if they don't exist
    init_node_state(
        &opts.state,
        &cmd.node_name,
        cmd.vault.as_deref(),
        cmd.identity.as_deref(),
    )
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
        "--project-identifier".to_string(),
        cmd.project_identifier.clone(),
        "--tcp-listener-address".to_string(),
        cmd.tcp_listener_address.clone(),
        "--foreground".to_string(),
        "--child-process".to_string(),
    ];

    if cmd.logging_to_file() || !opts.terminal.is_tty() {
        args.push("--no-color".to_string());
    }

    if cmd.no_direct_authentication {
        args.push("--no-direct-authentication".to_string());
    }

    if cmd.no_token_enrollment {
        args.push("--no-token-enrollment".to_string());
    }

    if let Some(trusted_identities) = &cmd.trusted_identities {
        args.push("--trusted-identities".to_string());
        args.push(trusted_identities.to_string());
    }

    if let Some(reload_from_trusted_identities_file) = &cmd.reload_from_trusted_identities_file {
        args.push("--reload-from-trusted-identities-file".to_string());
        args.push(
            reload_from_trusted_identities_file
                .to_string_lossy()
                .to_string(),
        );
    }

    if let Some(tenant_base_url) = &cmd.tenant_base_url {
        args.push("--tenant-base-url".to_string());
        args.push(tenant_base_url.clone());
    }

    if let Some(certificate) = &cmd.certificate {
        args.push("--certificate".to_string());
        args.push(certificate.clone());
    }

    if let Some(attributes) = &cmd.attributes {
        attributes.iter().for_each(|attr| {
            args.push("--attributes".to_string());
            args.push(attr.clone());
        });
    }

    if let Some(vault) = &cmd.vault {
        args.push("--vault".to_string());
        args.push(vault.clone());
    }

    if let Some(identity) = &cmd.identity {
        args.push("--identity".to_string());
        args.push(identity.clone());
    }
    args.push(cmd.node_name.to_string());

    run_ockam(opts, &cmd.node_name, args, cmd.logging_to_file())
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if self.foreground {
            // Create a new node in the foreground (i.e. in this OS process)
            local_cmd(embedded_node_that_is_not_stopped(
                start_authority_node,
                (options, self),
            ))
        } else {
            // Create a new node running in the background (i.e. another, new OS process)
            node_rpc(create_background_node, (options, self))
        }
    }

    /// Return a source of pre trusted identities and their attributes
    /// This is either a file which is used as the backend of the AttributesStorage
    /// or an explicit list of identities passed on the command line
    pub(crate) fn trusted_identities(
        &self,
        authority_identifier: &Identifier,
    ) -> Result<PreTrustedIdentities> {
        match (
            &self.reload_from_trusted_identities_file,
            &self.trusted_identities,
        ) {
            (Some(path), None) => Ok(PreTrustedIdentities::ReloadFrom(path.clone())),
            (None, Some(trusted)) => Ok(PreTrustedIdentities::Fixed(trusted.to_map(
                self.project_identifier.to_string(),
                authority_identifier,
            ))),
            _ => Err(crate::Error::new(
                exitcode::CONFIG,
                miette!("Exactly one of 'reload-from-trusted-identities-file' or 'trusted-identities' must be defined"),
            )),
        }
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
}

/// Given a Context start a node in a new OS process
async fn create_background_node(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    // Spawn node in another, new process
    spawn_background_node(&opts, &cmd).await
}

/// Start an authority node:
///   - retrieve the node identity if the authority identity has been created before
///   - persist the node state
///   - start the node services
async fn start_authority_node(
    ctx: Context,
    args: (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let (opts, cmd) = args;

    // Create node state, including the vault and identity if they don't exist
    if !opts.state.nodes.exists(&cmd.node_name) {
        init_node_state(
            &opts.state,
            &cmd.node_name,
            cmd.vault.as_deref(),
            cmd.identity.as_deref(),
        )
        .await?;
    };

    // Retrieve the authority identity if it has been created before
    // otherwise create a new one
    let identifier = match &cmd.identity {
        Some(identity_name) => {
            debug!(name=%identity_name, "getting identity from state");
            opts.state
                .identities
                .get(identity_name)
                .context("Identity not found")?
                .config()
                .identifier()
        }
        None => {
            debug!("getting default identity from state");
            match opts.state.identities.default() {
                Ok(state) => state.config().identifier(),
                Err(_) => {
                    debug!("creating default identity");
                    let cmd = identity::CreateCommand::new("authority".into(), None, None);
                    cmd.create_identity(opts.clone()).await?
                }
            }
        }
    };
    debug!(identifier=%identifier, "authority identifier");

    let okta_configuration = match (&cmd.tenant_base_url, &cmd.certificate, &cmd.attributes) {
        (Some(tenant_base_url), Some(certificate), Some(attributes)) => Some(OktaConfiguration {
            address: DefaultAddress::OKTA_IDENTITY_PROVIDER.to_string(),
            tenant_base_url: tenant_base_url.clone(),
            certificate: certificate.clone(),
            attributes: attributes.clone(),
        }),
        _ => None,
    };

    // persist the node state and mark it as an authority node
    // That flag allows the node to be seen as UP when listing the nodes with the
    // the `ockam node list` command, without having to send a TCP query to open a connection
    // because this would fail if there is no intention to create a secure channel
    debug!("updating node state's setup config");
    let node_state = opts.state.nodes.get(&cmd.node_name)?;
    node_state.set_setup(
        &node_state
            .config()
            .setup_mut()
            .set_verbose(opts.global_args.verbose)
            .set_authority_node()
            .set_api_transport(
                CreateTransportJson::new(
                    TransportType::Tcp,
                    TransportMode::Listen,
                    cmd.tcp_listener_address.as_str(),
                )
                .into_diagnostic()?,
            ),
    )?;

    let trusted_identities = cmd.trusted_identities(&identifier)?;

    let configuration = authority_node::Configuration {
        identifier,
        storage_path: opts.state.identities.identities_repository_path()?,
        vault_path: opts.state.vaults.default()?.vault_file_path().clone(),
        project_identifier: cmd.project_identifier,
        tcp_listener_address: cmd.tcp_listener_address,
        secure_channel_listener_name: None,
        authenticator_name: None,
        trusted_identities,
        no_direct_authentication: cmd.no_direct_authentication,
        no_token_enrollment: cmd.no_token_enrollment,
        okta: okta_configuration,
    };
    authority_node::start_node(&ctx, &configuration)
        .await
        .into_diagnostic()?;

    Ok(())
}

/// Return a list of trusted identities passed as a JSON string on the command line
fn parse_trusted_identities(values: &str) -> Result<TrustedIdentities> {
    serde_json::from_str::<TrustedIdentities>(values).map_err(|e| {
        crate::Error::new(
            exitcode::CONFIG,
            miette!("Cannot parse the trusted identities: {}", e),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::Identifier;
    use ockam_core::compat::collections::HashMap;

    #[test]
    fn test_parse_trusted_identities() {
        let identity1 = Identifier::try_from("Ie86be15e83d1c93e24dd1967010b01b6df491b45").unwrap();
        let identity2 = Identifier::try_from("I6c20e814b56579306f55c64e8747e6c1b4a53d9a").unwrap();

        let trusted = format!("{{\"{identity1}\": {{\"name\": \"value\", \"trust_context_id\": \"1\"}}, \"{identity2}\": {{\"trust_context_id\" : \"1\", \"ockam-role\" : \"enroller\"}}}}");
        let actual = parse_trusted_identities(trusted.as_str()).unwrap();

        let attributes1 = HashMap::from([
            ("name".into(), "value".into()),
            ("trust_context_id".into(), "1".into()),
        ]);
        let attributes2 = HashMap::from([
            ("trust_context_id".into(), "1".into()),
            ("ockam-role".into(), "enroller".into()),
        ]);
        let expected = vec![
            TrustedIdentity::new(&identity2, &attributes2),
            TrustedIdentity::new(&identity1, &attributes1),
        ];
        assert_eq!(actual.trusted_identities(), expected);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct TrustedIdentities(HashMap<Identifier, HashMap<String, String>>);

impl TrustedIdentities {
    pub fn trusted_identities(&self) -> Vec<TrustedIdentity> {
        self.0
            .iter()
            .map(|(k, v)| TrustedIdentity::new(k, v))
            .collect()
    }

    /// Return a map from Identifier to AttributesEntry and:
    ///   - add the project identifier as an attribute
    ///   - use the authority identifier an the attributes issuer
    pub(crate) fn to_map(
        &self,
        project_identifier: String,
        authority_identifier: &Identifier,
    ) -> HashMap<Identifier, AttributesEntry> {
        HashMap::from_iter(self.trusted_identities().iter().map(|t| {
            (
                t.identifier(),
                t.attributes_entry(project_identifier.clone(), authority_identifier),
            )
        }))
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
