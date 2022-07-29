use super::map_anyhow_err;
use crate::error::ApiError;
use crate::nodes::models::vault::CreateVaultRequest;
use crate::nodes::NodeManager;
use crate::{Request, Response, ResponseBuilder};
use minicbor::Decoder;
use ockam::vault::storage::FileStorage;
use ockam::vault::Vault;
use ockam::Result;
use std::path::PathBuf;
use std::sync::Arc;

impl NodeManager {
    pub(crate) fn vault(&self) -> Result<&Vault> {
        self.vault
            .as_ref()
            .ok_or_else(|| ApiError::generic("Vault doesn't exist"))
    }

    pub(super) async fn create_vault_impl(&mut self, path: Option<PathBuf>) -> Result<()> {
        if self.vault.is_some() {
            return Err(ApiError::generic("Vault already exists"))?;
        }

        let path = path.unwrap_or_else(|| self.node_dir.join("vault.json"));

        let vault_storage = FileStorage::create(path.clone()).await?;
        let vault = Vault::new(Some(Arc::new(vault_storage)));

        self.config.inner().write().unwrap().vault_path = Some(path);
        self.config.atomic_update().run().map_err(map_anyhow_err)?;

        self.vault = Some(vault);

        Ok(())
    }

    pub(super) async fn create_vault(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: CreateVaultRequest = dec.decode()?;

        let path = req_body.path.map(|p| PathBuf::from(p.0.as_ref()));

        self.create_vault_impl(path).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
