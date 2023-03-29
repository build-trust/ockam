use anyhow::anyhow;
use clap::Args;
use ockam::Context;
use ockam_api::trust_context::{
    AuthorityInfo, CredentialIssuerInfo, CredentialRetriever, TrustContext,
};
use ockam_identity::{credential::Credential, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use crate::{util::node_rpc, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct SerializeCommand {
    #[arg(long = "authority-identity")]
    pub authority_identity: String,

    #[arg(group = "own_credential", long, value_parser = parse_credential)]
    pub credential: Option<Credential>,

    #[arg(group = "own_credential", long)]
    pub credential_name: Option<String>,

    #[arg(group = "own_credential", long)]
    pub credential_issuer: Option<MultiAddr>,
}

impl SerializeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SerializeCommand),
) -> crate::Result<()> {
    let identity_as_bytes = match hex::decode(cmd.authority_identity) {
        Ok(b) => b,
        Err(e) => return Err(anyhow!(e).into()),
    };

    let public_identity = PublicIdentity::import(&identity_as_bytes, Vault::create()).await?;

    let own_cred = match (cmd.credential, cmd.credential_name, cmd.credential_issuer) {
        (Some(cred), None, None) => Some(CredentialRetriever::FromMemory(cred)),
        (None, Some(cred_name), None) => {
            let state = opts.state.credentials.get(&cred_name)?;

            Some(CredentialRetriever::FromState(state))
        }
        (None, None, Some(addr)) => Some(CredentialRetriever::FromCredentialIssuer(
            CredentialIssuerInfo::new(addr),
        )),
        _ => None,
    };

    let tc = TrustContext::new(AuthorityInfo::new(public_identity, own_cred));
    let tc_as_json = serde_json::to_string_pretty(&tc)?;
    println!("{tc_as_json}");

    Ok(())
}

pub fn parse_credential(credential: &str) -> Result<Credential> {
    let bytes = match hex::decode(credential) {
        Ok(b) => b,
        Err(e) => return Err(anyhow!(e).into()),
    };

    let cred: Credential = minicbor::decode(&bytes)?;

    Ok(cred)
}
