use crate::influxdb::influxdb_api_client::InfluxDBApi;
use crate::influxdb::lease_token::LeaseToken;
use crate::token_lessor_node_service::InfluxDBTokenLessorState;
use crate::ApiError;
use ockam_core::{async_trait, Processor};
use ockam_node::Context;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) struct InfluxDBTokenLessorProcessor {
    state: Arc<RwLock<InfluxDBTokenLessorState>>,
}

impl InfluxDBTokenLessorProcessor {
    pub(crate) fn new(state: Arc<RwLock<InfluxDBTokenLessorState>>) -> Self {
        Self { state }
    }

    async fn list_tokens(&self) -> ockam_core::Result<Vec<LeaseToken>> {
        debug!("Listing all tokens");
        let influxdb_tokens = {
            let state = self.state.read().await;
            state
                .influxdb_api_client
                .list_tokens()
                .await?
                .into_response()
                .map_err(|_| ApiError::core("Failed to list tokens"))?
                .tokens
        };
        let lease_tokens: Vec<LeaseToken> = influxdb_tokens
            .into_iter()
            .filter_map(|token| {
                let lease_token: Result<Option<LeaseToken>, _> = token.try_into();
                lease_token.ok().flatten()
            })
            .collect();
        info!("Found {} tokens", lease_tokens.len());
        Ok(lease_tokens)
    }

    async fn revoke_outstanding_tokens(&self) -> ockam_core::Result<()> {
        let to_remove = {
            let state = self.state.read().await;
            if state.active_tokens.is_empty() {
                return Ok(());
            }
            let influxdb_api_client = &state.influxdb_api_client;
            let mut to_remove = vec![];
            for token in state.active_tokens.iter() {
                if token.0.is_expired().unwrap_or(true) {
                    let token_id = &token.0.id;
                    if influxdb_api_client.revoke_token(token_id).await.is_ok() {
                        to_remove.push(token_id.clone());
                    }
                } else {
                    break;
                }
            }
            to_remove
        };
        {
            let mut state = self.state.write().await;
            state
                .active_tokens
                .retain(|token| !to_remove.contains(&token.0.id));
        }
        Ok(())
    }
}

#[async_trait]
impl Processor for InfluxDBTokenLessorProcessor {
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> ockam_core::Result<()> {
        let mut max_retries = 5;
        loop {
            match self.list_tokens().await {
                Ok(tokens) => {
                    let mut state = self.state.write().await;
                    state.active_tokens =
                        BinaryHeap::from(tokens.into_iter().map(Reverse).collect::<Vec<_>>());
                    break;
                }
                Err(_) => {
                    max_retries -= 1;
                    if max_retries == 0 {
                        error!("Failed to initialize InfluxDBTokenLessorProcessor");
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> ockam_core::Result<()> {
        debug!("Shutting down InfluxDBTokenLessorProcessor");
        Ok(())
    }

    async fn process(&mut self, _context: &mut Self::Context) -> ockam_core::Result<bool> {
        if let Err(err) = self.revoke_outstanding_tokens().await {
            error!("Failed to revoke outstanding tokens: {err}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(true)
    }
}
