use crate::authority::HELP_DETAIL;
use crate::help;
use crate::node::util::init_node_state;
use crate::node::util::run_ockam;
use crate::util::node_rpc;
use crate::util::{embedded_node_that_is_not_stopped, exitcode};
use crate::{identity, CommandGlobalOpts, Result};
use anyhow::anyhow;
use clap::{ArgGroup, Args};
use ockam::AsyncTryClone;
use ockam::Context;
use ockam_api::nodes::authority_node;
use ockam_api::nodes::authority_node::{OktaConfiguration, TrustedIdentity};
use ockam_api::DefaultAddress;
use ockam_core::compat::fmt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tracing::error;

/// Create a node
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
#[clap(group(ArgGroup::new("okta").args(&["tenant_base_url", "certificate", "attributes"])))]
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

    /// List of the trusted identities, and corresponding attributes to be preload in the attributes storage
    #[arg(long, value_name = "JSON_ARRAY", value_parser=parse_trusted_identities)]
    trusted_identities: TrustedIdentities,

    /// Okta: URL used for accessing the Okta API (optional)
    #[arg(long, group = "okta", value_name = "URL", default_value = None)]
    tenant_base_url: Option<String>,

    /// Okta: pem certificate used to access the Okta server (optional)
    #[arg(long, group = "okta", value_name = "STRING", default_value = None)]
    certificate: Option<String>,

    /// Okta: name of the attributes which can be retrieved from Okta (optional)
    #[arg(long, group = "okta", value_name = "COMMA_SEPARATED_LIST", default_value = None)]
    attributes: Option<Vec<String>>,

    /// Run the node in foreground.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    foreground: bool,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if self.foreground {
            // Create a new node in the foreground (i.e. in this OS process)
            if let Err(e) = embedded_node_that_is_not_stopped(start_authority_node, (options, self))
            {
                error!(%e);
                eprintln!("{e:?}");
                std::process::exit(e.code());
            }
        } else {
            // Create a new node running in the background (i.e. another, new OS process)
            node_rpc(create_background_node, (options, self))
        }
    }
}

async fn create_background_node(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    // Spawn node in another, new process
    spawn_background_node(&ctx, &opts, &cmd).await
}

async fn spawn_background_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
) -> crate::Result<()> {
    // Create node state, including the vault and identity if don't exist
    init_node_state(ctx, opts, &cmd.node_name, None, None).await?;

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    let mut args = vec![
        "authority".to_string(),
        "create".to_string(),
        "--project-identifier".to_string(),
        cmd.project_identifier.clone(),
        "--tcp-listener-address".to_string(),
        cmd.tcp_listener_address.clone(),
        "--trusted-identities".to_string(),
        cmd.trusted_identities.to_string(),
        "--foreground".to_string(),
    ];

    if let Some(tenant_base_url) = &cmd.tenant_base_url {
        args.push("--tenant-base-url".to_string());
        args.push(tenant_base_url.clone());
    }

    if let Some(certificate) = &cmd.certificate {
        args.push("--certificate".to_string());
        args.push(certificate.clone());
    }

    if let Some(attributes) = &cmd.attributes {
        args.push("--attributes".to_string());
        args.push(attributes.join(","));
    }
    args.push(cmd.node_name.to_string());

    run_ockam(opts, &cmd.node_name, args)
}

async fn start_authority_node(
    ctx: Context,
    opts: (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let (options, cmd) = opts;
    let command = cmd.clone();

    // retrieve the authority identity if it has been created before
    // otherwise create a new one
    let public_identity = match options.state.identities.default().ok() {
        Some(state) => state.config.public_identity(),
        None => {
            let cmd = identity::CreateCommand::new("authority".into(), None);
            cmd.create_identity(ctx.async_try_clone().await?, options.clone())
                .await?
        }
    };

    let okta_configuration = match (
        command.tenant_base_url,
        command.certificate,
        command.attributes,
    ) {
        (Some(tenant_base_url), Some(certificate), Some(attributes)) => Some(OktaConfiguration {
            address: DefaultAddress::OKTA_IDENTITY_PROVIDER.to_string(),
            tenant_base_url,
            certificate,
            attributes,
        }),
        _ => None,
    };

    let configuration = authority_node::Configuration {
        identity: public_identity,
        storage_path: options.state.identities.authenticated_storage_path()?,
        vault_path: options.state.vaults.default()?.vault_file_path()?,
        project_identifier: command.project_identifier,
        tcp_listener_address: command.tcp_listener_address.clone(),
        secure_channel_listener_name: None,
        authenticator_name: None,
        trusted_identities: command.trusted_identities.0,
        okta: okta_configuration,
    };
    authority_node::start_node(&ctx, &configuration).await?;

    Ok(())
}

fn parse_trusted_identities(values: &str) -> Result<TrustedIdentities> {
    serde_json::from_str::<TrustedIdentities>(values).map_err(|e| {
        crate::Error::new(
            exitcode::CONFIG,
            anyhow!("Cannot parse the trusted identities: {e}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::compat::collections::HashMap;
    use ockam_identity::IdentityIdentifier;
    use std::str::FromStr;

    #[test]
    fn test_parse_trusted_identities() {
        let identity1 = IdentityIdentifier::from_str(
            "Pe86be15e83d1c93e24dd1967010b01b6df491b459725fd9ae0bebfd7c1bf8ea3",
        )
        .unwrap();
        let identity2 = IdentityIdentifier::from_str(
            "P6c20e814b56579306f55c64e8747e6c1b4a53d9a3f4ca83c252cc2fbfc72fa94",
        )
        .unwrap();

        let trusted = format!("[{{\"identifier\":\"{identity1}\", \"attributes\": {{\"name\" : \"value\", \"project_id\" : \"1\"}}}}, {{\"identifier\":\"{identity2}\", \"attributes\": {{\"project_id\" : \"1\", \"ockam-role\" : \"enroller\"}}}}]");
        let actual = parse_trusted_identities(&trusted.as_str()).unwrap();

        let attributes1 = HashMap::from([
            ("name".into(), "value".into()),
            ("project_id".into(), "1".into()),
        ]);
        let attributes2 = HashMap::from([
            ("project_id".into(), "1".into()),
            ("ockam-role".into(), "enroller".into()),
        ]);
        let expected = vec![
            TrustedIdentity::new(&identity1, &attributes1),
            TrustedIdentity::new(&identity2, &attributes2),
        ];
        assert_eq!(actual.0, expected);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct TrustedIdentities(Vec<TrustedIdentity>);

impl Display for TrustedIdentities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| fmt::Error)?
                .as_str(),
        )
    }
}
