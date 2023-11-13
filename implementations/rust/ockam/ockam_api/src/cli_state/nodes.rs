use std::path::PathBuf;
use std::process;

use nix::errno::Errno;
use sysinfo::{Pid, ProcessExt, ProcessStatus, System, SystemExt};

use ockam::identity::{Identifier, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_multiaddr::MultiAddr;

use crate::cli_state::{random_name, Result};
use crate::cli_state::{CliState, CliStateError};
use crate::cloud::project::Project;
use crate::config::lookup::InternetAddress;

impl CliState {
    /// Create a node with an identity (possibly associated with a name) and an optionally specified vault
    pub async fn create_node_with_optional_name_and_optional_vault_and_optional_project(
        &self,
        node_name: &Option<String>,
        identity_name: &Option<String>,
        vault_name: &Option<String>,
        project_name: &Option<String>,
    ) -> Result<NodeInfo> {
        let node_name = self.get_node_name_or_default(node_name).await?;
        let vault_name = self.get_vault_name_or_default(vault_name).await?;
        let identity_name = self.get_identity_name_or_default(identity_name).await?;
        // note that the identity is created only if it has not been created before
        let identifier = self
            .create_identity_with_name_and_vault(&identity_name, &vault_name)
            .await?;
        let node = self
            .create_node_with_identifier(&node_name, &identifier)
            .await?;
        self.set_node_project(&node_name, project_name).await?;
        Ok(node)
    }

    /// This method creates a node with an associated identity
    /// The vault used to create the identity is the default vault
    pub async fn create_node(&self, node_name: &str) -> Result<NodeInfo> {
        let identifier = self.create_identity_with_name(&random_name()).await?;
        self.create_node_with_identifier(node_name, &identifier)
            .await
    }

    pub async fn create_node_with_identifier(
        &self,
        node_name: &str,
        identifier: &Identifier,
    ) -> Result<NodeInfo> {
        let repository = self.nodes_repository().await?;
        let is_default = repository.is_default_node(node_name).await?
            || repository.get_nodes().await?.is_empty();
        let tcp_listener_address = repository.get_tcp_listener_address(node_name).await?;
        let node_info = NodeInfo::new(
            node_name.to_string(),
            identifier.clone(),
            0,
            is_default,
            false,
            tcp_listener_address,
            Some(process::id()),
        );
        repository.store_node(&node_info).await?;
        Ok(node_info)
    }

    pub async fn store_node(&self, node_info: &NodeInfo) -> Result<()> {
        Ok(self.nodes_repository().await?.store_node(node_info).await?)
    }

    pub async fn get_node(&self, node_name: &str) -> Result<NodeInfo> {
        if let Some(node) = self.nodes_repository().await?.get_node(node_name).await? {
            Ok(node)
        } else {
            Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("There is no node with name {node_name}"),
            )
            .into())
        }
    }

    /// Return all the registered nodes
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        Ok(self.nodes_repository().await?.get_nodes().await?)
    }

    /// Return the identifier associated to a node
    pub async fn get_node_identifier(&self, node_name: &str) -> Result<Identifier> {
        Ok(self.get_node(node_name).await?.identifier())
    }

    /// Return true if that node is the default one
    pub async fn is_default_node(&self, node_name: &str) -> Result<bool> {
        Ok(self.get_node(node_name).await?.is_default())
    }

    /// Return true if that node is currently running
    pub async fn is_node_running(&self, node_name: &str) -> Result<bool> {
        Ok(self
            .get_node(node_name)
            .await
            .ok()
            .map(|n| n.is_running())
            .unwrap_or(false))
    }

    /// Return true if that node is an authority node
    pub async fn is_authority_node(&self, node_name: &str) -> Result<bool> {
        Ok(self
            .get_node(node_name)
            .await
            .ok()
            .map(|n| n.is_authority_node())
            .unwrap_or(false))
    }

    /// Return the name of the identifier associated to a node
    pub async fn get_node_identifier_name(&self, node_name: &str) -> Result<Option<String>> {
        let identifier = self.get_node_identifier(node_name).await?;
        Ok(self
            .identities_repository()
            .await?
            .get_identity_name_by_identifier(&identifier)
            .await?)
    }

    /// Return information about the default node (if there is one)
    pub async fn get_default_node(&self) -> Result<NodeInfo> {
        if let Some(node) = self.nodes_repository().await?.get_default_node().await? {
            Ok(node)
        } else {
            let identity = self.get_default_named_identity().await?;
            let node = self
                .create_node_with_identifier(&random_name(), &identity.identifier())
                .await?;
            Ok(node)
        }
    }

    pub async fn set_default_node(&self, node_name: &str) -> Result<()> {
        Ok(self
            .nodes_repository()
            .await?
            .set_default_node(node_name)
            .await?)
    }

    pub async fn set_tcp_listener_address(&self, node_name: &str, address: String) -> Result<()> {
        Ok(self
            .nodes_repository()
            .await?
            .set_tcp_listener_address(node_name, address.as_str())
            .await?)
    }

    pub async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()> {
        Ok(self
            .nodes_repository()
            .await?
            .set_node_pid(node_name, pid)
            .await?)
    }

    /// Return the node_name if Some otherwise return the default node name (if there is one)
    pub async fn get_node_name_or_default(&self, node_name: &Option<String>) -> Result<String> {
        match node_name {
            Some(name) => Ok(name.clone()),
            None => self.get_default_node_name().await,
        }
    }

    /// Return the node information for the given node name, otherwise for the default node
    pub async fn get_node_or_default(&self, node_name: &Option<String>) -> Result<NodeInfo> {
        match node_name {
            Some(name) => self.get_node(name).await,
            None => self.get_default_node().await,
        }
    }

    /// Return the default node name.
    /// If there is no existing default node, return a constant name to use as the default
    pub async fn get_default_node_name(&self) -> Result<String> {
        self.get_default_node().await.map(|n| n.name())
    }

    /// Return the vault which was used to create the identity associated to a node
    pub async fn get_node_vault(&self, node_name: &str) -> Result<Vault> {
        let identifier = self.get_node_identifier(node_name).await?;
        let named_vault = self.get_identifier_vault(&identifier).await?;
        named_vault.vault().await
    }

    pub fn stdout_logs(&self, node_name: &str) -> Result<PathBuf> {
        Ok(self.create_node_dir(node_name)?.join("stdout.log"))
    }

    pub fn stderr_logs(&self, node_name: &str) -> Result<PathBuf> {
        Ok(self.create_node_dir(node_name)?.join("stderr.log"))
    }

    pub fn create_node_dir(&self, node_name: &str) -> Result<PathBuf> {
        let path = self.make_node_dir(node_name);
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn make_node_dir(&self, node_name: &str) -> PathBuf {
        Self::make_node_dir_path(&self.dir(), node_name)
    }

    /// Delete all registered nodes
    pub async fn delete_all_nodes(&self, force: bool) -> Result<()> {
        let nodes = self.nodes_repository().await?.get_nodes().await?;
        for node in nodes {
            self.delete_node(&node.name(), force).await?;
        }
        Ok(())
    }

    /// Delete the default node if there is one
    pub async fn delete_default_node(&self, force: bool) -> Result<()> {
        let node_name = self.get_default_node().await?.name();
        self.delete_node(&node_name, force).await
    }

    pub async fn delete_node(&self, node_name: &str, force: bool) -> Result<()> {
        self.stop_node(node_name, force).await?;
        self.remove_node(node_name).await?;
        Ok(())
    }

    /// Remove a node
    pub async fn remove_node(&self, node_name: &str) -> Result<()> {
        // don't try to remove a node on a non-existent database
        if !self.database_path().exists() {
            return Ok(());
        };

        // remove the node from the database
        let repository = self.nodes_repository().await?;
        let node_exists = repository.get_node(node_name).await.is_ok();
        repository.delete_node(node_name).await?;

        // set another node as the default node
        if node_exists {
            let other_nodes = repository.get_nodes().await?;
            if let Some(other_node) = other_nodes.first() {
                repository.set_default_node(&other_node.name()).await?;
            }
        }

        // remove the node directory
        let _ = std::fs::remove_dir_all(self.make_node_dir(node_name));
        debug!(name=%node_name, "node deleted");
        Ok(())
    }

    pub async fn stop_node(&self, node_name: &str, force: bool) -> Result<()> {
        let node = self.get_node(node_name).await?;
        self.nodes_repository()
            .await?
            .set_no_node_pid(node_name)
            .await?;

        if let Some(pid) = node.pid() {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                if force {
                    nix::sys::signal::Signal::SIGKILL
                } else {
                    nix::sys::signal::Signal::SIGTERM
                },
            )
            .or_else(|e| {
                if e == Errno::ESRCH {
                    tracing::warn!(node = %node.name(), %pid, "No such process");
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|e| {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to stop PID `{pid}` with error `{e}`"),
                ))
            })?;
        }
        info!(name = %node.name(), "node process killed");
        Ok(())
    }

    pub async fn get_node_project(&self, node_name: &str) -> Result<Project> {
        match self
            .nodes_repository()
            .await?
            .get_node_project_name(node_name)
            .await?
        {
            Some(project_name) => self.get_project_by_name(&project_name).await,
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no project associated to node {node_name}"),
            )
            .into()),
        }
    }

    pub async fn set_node_project(
        &self,
        node_name: &str,
        project_name: &Option<String>,
    ) -> Result<()> {
        let project = match project_name {
            Some(name) => Some(self.get_project_by_name(name).await?),
            None => self.get_default_project().await.ok(),
        };

        if let Some(project) = project {
            self.nodes_repository()
                .await?
                .set_node_project_name(node_name, &project.name())
                .await?
        };
        Ok(())
    }

    pub async fn get_node_by_identity_name(&self, identity_name: &str) -> Result<Option<NodeInfo>> {
        let identifier = self.get_identifier_by_name(identity_name).await?;
        self.get_node_by_identifier(&identifier).await
    }

    pub async fn get_node_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<NodeInfo>> {
        Ok(self
            .nodes_repository()
            .await?
            .get_node_by_identifier(identifier)
            .await?)
    }
}

/// This struct contains all the data associated to a node
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NodeInfo {
    name: String,
    identifier: Identifier,
    verbosity: u8,
    // this is used when restarting the node to determine its logging level
    is_default: bool,
    is_authority: bool,
    tcp_listener_address: Option<InternetAddress>,
    pid: Option<u32>,
}

impl NodeInfo {
    pub fn new(
        name: String,
        identifier: Identifier,
        verbosity: u8,
        is_default: bool,
        is_authority: bool,
        tcp_listener_address: Option<InternetAddress>,
        pid: Option<u32>,
    ) -> Self {
        Self {
            name,
            identifier,
            verbosity,
            is_default,
            is_authority,
            tcp_listener_address,
            pid,
        }
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Return a copy of this node with the is_default flag set to true
    pub fn set_as_default(&self) -> Self {
        let mut result = self.clone();
        result.is_default = true;
        result
    }

    pub fn is_authority_node(&self) -> bool {
        self.is_authority
    }

    pub fn tcp_listener_port(&self) -> Option<u16> {
        self.tcp_listener_address.as_ref().map(|t| t.port())
    }

    pub fn tcp_listener_address(&self) -> Option<InternetAddress> {
        self.tcp_listener_address.clone()
    }

    pub fn tcp_listener_multi_address(&self) -> Result<MultiAddr> {
        Ok(self
            .tcp_listener_address
            .as_ref()
            .ok_or(ockam::Error::new(
                Origin::Api,
                Kind::Internal,
                "no transport has been set on the node".to_string(),
            ))
            .and_then(|t| t.multi_addr())?)
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Return true if there is a running process corresponding to the node process id
    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.pid() {
            let mut sys = System::new();
            sys.refresh_processes();
            if let Some(p) = sys.process(Pid::from(pid as usize)) {
                // Under certain circumstances the process can be in a state where it's not running
                // and we are unable to kill it. For example, `kill -9` a process created by
                // `node create` in a Docker environment will result in a zombie process.
                !matches!(p.status(), ProcessStatus::Dead | ProcessStatus::Zombie)
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::lookup::InternetAddress;

    use super::*;

    #[tokio::test]
    async fn test_create_node() -> Result<()> {
        let cli = CliState::test().await?;

        // a node can be created with just a name
        let node_name = "node-1";
        let result = cli.create_node(node_name).await?;
        assert_eq!(result.name(), node_name.to_string());

        // the first node is the default one
        let result = cli.get_default_node_name().await?;
        assert_eq!(result, node_name.to_string());

        // as a consequence, a default identity must have been created
        let result = cli.get_default_vault().await.ok();
        assert!(result.is_some());

        let result = cli.get_default_named_identity().await.ok();
        assert!(result.is_some());

        // that identity is associated to the node
        let identifier = result.unwrap().identifier();
        let result = cli.get_node_identifier(node_name).await?;
        assert_eq!(result, identifier);
        Ok(())
    }

    #[tokio::test]
    async fn test_update_node() -> Result<()> {
        let cli = CliState::test().await?;

        // create a node
        let node_name = "node-1";
        let _ = cli.create_node(node_name).await?;
        cli.set_tcp_listener_address(node_name, "127.0.0.1:0".to_string())
            .await?;

        // recreate the node with the same name
        let _ = cli.create_node(node_name).await?;

        // the node must still be the default node
        let result = cli.get_default_node().await?;
        assert_eq!(result.name(), node_name.to_string());
        assert!(result.is_default());

        // the original tcp listener address has been kept
        assert_eq!(
            result.tcp_listener_address(),
            InternetAddress::new("127.0.0.1:0")
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_remove_node() -> Result<()> {
        let cli = CliState::test().await?;

        // a node can be created with just a name
        let node1 = "node-1";
        let node_info1 = cli.create_node(node1).await?;

        // the created node is set as the default node
        let result = cli.get_default_node().await?;
        assert_eq!(result, node_info1);

        // a node can also be removed
        // first let's create a second node
        let node2 = "node-2";
        let node_info2 = cli.create_node(node2).await?;

        // and remove node 1
        cli.remove_node(node1).await?;

        let result = cli.get_node(node1).await.ok();
        assert_eq!(
            result, None,
            "the node information is not available anymore"
        );
        assert!(
            !cli.make_node_dir(node1).exists(),
            "the node directory must be deleted"
        );

        // then node 2 should be the default node
        let result = cli.get_default_node().await?;
        assert_eq!(result, node_info2.set_as_default());
        Ok(())
    }

    #[tokio::test]
    async fn test_create_node_with_optional_name_and_optional_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // a node can be created with no name
        let node = cli
            .create_node_with_optional_name_and_optional_vault_and_optional_project(
                &None, &None, &None, &None,
            )
            .await?;
        let result = cli.get_default_node().await?;
        assert_eq!(result.name(), node.name());

        // a node can be created with just a name
        let node = cli
            .create_node_with_optional_name_and_optional_vault_and_optional_project(
                &Some("node-1".to_string()),
                &None,
                &None,
                &None,
            )
            .await?;
        let result = cli.get_node("node-1").await?;
        assert_eq!(result.name(), node.name());

        Ok(())
    }
}
