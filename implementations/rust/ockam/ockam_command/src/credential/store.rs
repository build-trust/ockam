use std::path::PathBuf;

use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::CommandGlobalOpts;
use ockam::identity::{CredentialRepository, CredentialSqlxDatabase, Identifier};
use ockam::Context;
use ockam_api::{fmt_log, fmt_ok};

use crate::credential::verify::verify_credential;
use crate::node::util::initialize_default_node;
use crate::node::NodeOpts;
use crate::util::parsers::identity_identifier_parser;

#[derive(Clone, Debug, Args)]
pub struct StoreCommand {
    #[arg(long = "issuer", value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub issuer: Identifier,

    /// Scope is used to separate credentials given they have the same Issuer&Subject Identifiers
    /// Scope can be an arbitrary value, however project admin, project member, and account admin
    /// credentials have scope of a specific format. See [`CredentialScope`]
    #[arg(long = "scope", value_name = "CREDENTIAL_SCOPE")]
    pub scope: String,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    /// Store the identity attributes of the credential for the specified node
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

impl StoreCommand {
    pub fn name(&self) -> String {
        "credential store".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        // Set node name in state to store identity attributes to it
        initialize_default_node(ctx, &opts).await?;
        let node_name = opts
            .state
            .get_node_or_default(&self.node_opts.at_node)
            .await?
            .name();
        let state = opts.state.clone();
        let database = state.database();
        let storage = CredentialSqlxDatabase::new(database, &node_name);

        opts.terminal
            .write_line(fmt_log!("Storing credential...\n"))?;

        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let credential = match verify_credential(
                &opts,
                &self.issuer,
                &self.credential,
                &self.credential_path,
            )
            .await
            {
                Ok(credential) => credential,
                Err(_err) => {
                    *is_finished.lock().await = true;
                    return Err(miette!("Credential is not verified"))?;
                }
            };

            let credential_data = credential
                .credential
                .get_credential_data()
                .map_err(|_| miette!("Invalid credential"))?;
            let purpose_key_data = credential
                .purpose_key_attestation
                .get_attestation_data()
                .map_err(|_| miette!("Invalid credential"))?;

            let subject = match credential_data.subject {
                None => {
                    *is_finished.lock().await = true;
                    return Err(miette!("credential subject is missing"))?;
                }
                Some(subject) => subject,
            };

            // store
            storage
                .put(
                    &subject,
                    &purpose_key_data.subject,
                    &self.scope,
                    credential_data.expires_at,
                    credential.clone(),
                )
                .await
                .map_err(|_e| miette!("Invalid credential"))?;

            *is_finished.lock().await = true;
            credential
                .encode_as_string()
                .map_err(|_e| miette!("Invalid credential"))
        };

        let output_messages = vec![format!("Storing credential...")];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (credential, _) = try_join!(send_req, progress_output)?;

        opts.terminal
            .stdout()
            .machine(credential.clone())
            .json(serde_json::json!(
                {
                    "issuer": self.issuer,
                    "credential": credential
                }
            ))
            .plain(fmt_ok!("Credential stored\n"))
            .write_line()?;

        Ok(())
    }
}
