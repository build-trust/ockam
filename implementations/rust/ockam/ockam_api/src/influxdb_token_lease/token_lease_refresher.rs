use chrono::DateTime;
use ockam::{compat::time::now, Address, Mailboxes};
use ockam_core::{api::Error, AllowAll, DenyAll};
use ockam_node::Context;
use std::{cmp::max, sync::Arc, time::Duration};
use tokio::sync::RwLock;

use crate::{
    cloud::{CredentialsEnabled, ProjectNodeClient},
    nodes::InMemoryNode,
    InfluxDbTokenLease,
};

// The default timeouts are too high.  Set shorter timeouts, so if
// the project is unresponsible or connection got lost for some reason
// (like in a project restart), it recover faster.
const SECURE_CHANNEL_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct TokenLeaseRefresher {
    token: Arc<RwLock<Option<String>>>,
}

impl TokenLeaseRefresher {
    pub async fn new(
        ctx: &Context,
        node_manager: Arc<InMemoryNode>,
    ) -> Result<TokenLeaseRefresher, Error> {
        let token = Arc::new(RwLock::new(None));
        let project = node_manager
            .cli_state
            .projects()
            .get_default_project()
            .await
            .map_err(|e| {
                Error::new_without_path().with_message(format!("No default project {}", e))
            })?;
        let project_identifier = project
            .project_identifier()
            .ok_or(Error::new_without_path().with_message("Project not configured"))?;
        let project_multiaddr = project.project_multiaddr().map_err(|e| {
            Error::new_without_path()
                .with_message(format!("Project multiaddr not configured {:}", e))
        })?;
        let identity_name = node_manager
            .cli_state
            .get_default_identity_name()
            .await
            .map_err(|e| {
                Error::new_without_path().with_message(format!("Error retrieving identity {:}", e))
            })?;
        let caller_identifier = node_manager
            .cli_state
            .get_identifier_by_name(&identity_name)
            .await
            .map_err(|e| {
                Error::new_without_path()
                    .with_message(format!("Error retrieving identifier {:}", e))
            })?;

        let project_client = node_manager
            .make_project_node_client(
                &project_identifier,
                project_multiaddr,
                &caller_identifier,
                CredentialsEnabled::On,
            )
            .await
            .map_err(|e| {
                Error::new_without_path()
                    .with_message(format!("Error creating project client {:}", e))
            })?
            .with_request_timeout(&REQUEST_TIMEOUT)
            .with_secure_channel_timeout(&SECURE_CHANNEL_TIMEOUT);
        let mailboxes = Mailboxes::main(
            Address::random_tagged("LeaseRetriever"),
            Arc::new(DenyAll),
            Arc::new(AllowAll),
        );
        let new_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;

        let token_clone = token.clone();
        ockam_node::spawn(async move {
            // TODO should it just loop again?
            if let Err(err) = refresh_loop(token_clone, new_ctx, project_client).await {
                error!("Token refresher terminated with error: {:}", err);
            }
        });
        Ok(Self { token })
    }

    pub async fn get_token(&self) -> Option<String> {
        self.token.read().await.clone()
    }
}

async fn refresh_loop(
    token: Arc<RwLock<Option<String>>>,
    ctx: Context,
    project_client: ProjectNodeClient,
) -> Result<(), Error> {
    loop {
        debug!("refreshing token");
        let token_result = project_client.create_token(&ctx).await;
        let now_t = now()?;
        let wait_secs = match token_result {
            Ok(new_token) => {
                let expires = DateTime::parse_from_rfc3339(&new_token.expires).map_err(|e| {
                Error::new_without_path()
                    .with_message(format!("Can't parse the expiration date for the just created token. Something is broken {:}", e))
                })?;
                let expires_unix = expires.timestamp() as u64;
                let duration = expires_unix - now_t;

                info!("Auth Token obtained expires at {}", new_token.expires);
                let mut t = token.write().await;
                *t = Some(new_token.token);
                // We request a new token once reaching half its duration, with a minimum
                // of 5 seconds.
                max(duration / 2, 5)
            }
            Err(err) => {
                warn!("Error retrieving token {:}", err);
                15
            }
        };
        debug!("waiting for {} seconds before refreshing token", wait_secs);
        ctx.sleep_long_until(now_t + wait_secs).await;
    }
}
