use crate::influxdb::lease_issuer::node_service::InfluxDBTokenLessorNodeServiceTrait;
use crate::nodes::InMemoryNode;
use ockam::{compat::time::now, Address, Mailboxes};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{api::Error, AllowAll, DenyAll};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::cmp::max;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TokenLeaseRefresher {
    token: Arc<RwLock<Option<String>>>,
}

impl TokenLeaseRefresher {
    pub fn new_with_fixed_token(token: String) -> TokenLeaseRefresher {
        let token = Arc::new(RwLock::new(Some(token)));
        Self { token }
    }
    pub fn new(
        ctx: &Context,
        node_manager: Weak<InMemoryNode>,
        lease_issuer_route: MultiAddr,
    ) -> Result<TokenLeaseRefresher, Error> {
        let token = Arc::new(RwLock::new(None));
        let mailboxes = Mailboxes::primary(
            Address::random_tagged("LeaseRetriever"),
            Arc::new(DenyAll),
            Arc::new(AllowAll),
        );
        let new_ctx = ctx.new_detached_with_mailboxes(mailboxes)?;

        let token_clone = token.clone();
        ockam_node::spawn(async move {
            // TODO should it just loop again?
            if let Err(err) = refresh_loop(
                token_clone,
                new_ctx,
                node_manager.clone(),
                lease_issuer_route,
            )
            .await
            {
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
    node_manager: Weak<InMemoryNode>,
    lease_issuer_route: MultiAddr,
) -> ockam_core::Result<()> {
    loop {
        debug!("refreshing token");
        let node_manager = node_manager.upgrade().ok_or_else(|| {
            ockam_core::Error::new(Origin::Node, Kind::Internal, "node manager was shut down")
        })?;
        let token_result = node_manager.create_token(&ctx, &lease_issuer_route).await;
        let now_t = now()?;
        let wait_secs = match token_result {
            Ok(new_token) => {
                let duration = new_token.expires_at as u64 - now_t;
                debug!("Auth Token obtained expires at {}", new_token.expires_at);
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
