use super::{map_anyhow_err, NodeManagerWorker};
use crate::nodes::models::vault::CreateVaultRequest;
use crate::nodes::NodeManager;
use minicbor::Decoder;
use ockam::vault::storage::FileStorage;
use ockam::vault::Vault;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::errcode::{Kind, Origin};
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl NodeManager {
    pub fn default_vault_path(node_dir: &Path) -> PathBuf {
        node_dir.join("vault.json")
    }

    pub(super) async fn create_vault_impl(
        &mut self,
        path: Option<PathBuf>,
        reuse_if_exists: bool,
    ) -> Result<()> {
        if self.vault.is_some() {
            return if reuse_if_exists {
                debug!("Using existing vault");
                Ok(())
            } else {
                Err(ockam_core::Error::new(
                    Origin::Application,
                    Kind::AlreadyExists,
                    "Vault already exists",
                ))
            };
        }

        let path = path.unwrap_or_else(|| Self::default_vault_path(&self.node_dir));

        let vault_storage = FileStorage::create(path.clone()).await?;
        let vault = Vault::new(Some(Arc::new(vault_storage)));

        let state = self.config.state();
        state.write().vault_path = Some(path);
        state.persist_config_updates().map_err(map_anyhow_err)?;

        self.vault = Some(vault);

        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) async fn create_vault(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: CreateVaultRequest = dec.decode()?;

        let path = req_body.path.map(|p| PathBuf::from(p.0.as_ref()));

        node_manager.create_vault_impl(path, false).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
