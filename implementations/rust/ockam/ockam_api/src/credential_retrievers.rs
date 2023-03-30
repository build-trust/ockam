use crate::error::ApiError;
use crate::{create_tcp_session, DefaultAddress};
use ockam::{route, TcpTransport};
use ockam_identity::credential::Credential;
use ockam_identity::trust_context::CredentialRetriever;
use ockam_identity::Identity;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;

use crate::authenticator::direct::CredentialIssuerClient;
use crate::authenticator::direct::RpcClient;
use ockam_identity::{SecureChannelTrustOptions, TrustMultiIdentifiersPolicy};
use std::time::Duration;

/// Retrieve credentials from an online authority
pub struct FromCredentialIssuer {
    authority_identifier: IdentityIdentifier,
    addr: MultiAddr,
    tcp_transport: TcpTransport,
}

impl FromCredentialIssuer {
    pub fn new(
        authority_identifier: IdentityIdentifier,
        addr: MultiAddr,
        tcp_transport: TcpTransport,
    ) -> FromCredentialIssuer {
        Self {
            authority_identifier,
            addr,
            tcp_transport,
        }
    }
}

#[ockam_core::async_trait]
impl CredentialRetriever for FromCredentialIssuer {
    async fn credential(&self, identity: &Identity) -> Result<Credential, ockam_core::Error> {
        debug!("Getting credential from : {}", self.addr);

        let allowed = vec![self.authority_identifier.clone()];

        let authority_tcp_session = match create_tcp_session(&self.addr, &self.tcp_transport).await
        {
            Some(authority_tcp_session) => authority_tcp_session,
            None => {
                let err_msg = format!("Invalid route within trust context: {}", &self.addr);
                error!("{err_msg}");
                return Err(ApiError::generic(&err_msg));
            }
        };

        debug!("Create secure channel to authority");

        let trust_options = SecureChannelTrustOptions::new();

        let trust_options = match authority_tcp_session.session {
            Some((sessions, session_id)) => trust_options.as_consumer(&sessions, &session_id),
            None => trust_options,
        };

        let trust_options =
            trust_options.with_trust_policy(TrustMultiIdentifiersPolicy::new(allowed));

        let sc = identity
            .create_secure_channel_extended(
                authority_tcp_session.route,
                trust_options,
                Duration::from_secs(120),
            )
            .await?;

        debug!("Created secure channel to project authority");

        let client = CredentialIssuerClient::new(
            RpcClient::new(
                route![sc, DefaultAddress::CREDENTIAL_ISSUER],
                identity.ctx(),
            )
            .await?,
        );

        let credential = client.credential().await?;

        //PABLO: TODO:  should verify the credential is correctly signed by authority here,
        //              for that we must have authority publicidentity , not just the identifier
        //identity
        //    .verify_self_credential(&credential, authorities.public_identities().iter())
        //    .await?;

        debug!("Verified self credential");

        Ok(credential)
        /*
        Err(ApiError::generic(
                    "Not Implemented",
                ))
        */
    }
}
