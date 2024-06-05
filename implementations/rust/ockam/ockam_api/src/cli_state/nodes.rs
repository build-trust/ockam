use minicbor::{Decode, Encode};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process;

use nix::errno::Errno;

use nix::sys::signal;
use serde::Serialize;
use sysinfo::{Pid, ProcessStatus, System};

use ockam::identity::utils::now;
use ockam::identity::Identifier;
use ockam::tcp::TcpListener;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_multiaddr::proto::{DnsAddr, Node, Tcp};
use ockam_multiaddr::MultiAddr;

use crate::cli_state::{random_name, NamedVault, Result};
use crate::cli_state::{CliState, CliStateError};
use crate::cloud::project::Project;
use crate::colors::color_primary;
use crate::config::lookup::InternetAddress;

use crate::ConnectionStatus;

/// The methods below support the creation and update of local nodes
impl CliState {
    /// Create a node, with some optional associated values, and start it
    #[instrument(skip_all, fields(node_name = node_name, identity_name = identity_name.clone(), project_name = project_name.clone()))]
    pub async fn start_node_with_optional_values(
        &self,
        node_name: &str,
        identity_name: &Option<String>,
        project_name: &Option<String>,
        tcp_listener: Option<&TcpListener>,
    ) -> Result<NodeInfo> {
        let mut node = self
            .create_node_with_optional_values(node_name, identity_name, project_name)
            .await?;
        if node.pid.is_none() {
            let pid = process::id();
            self.set_node_pid(node_name, pid).await?;
            node = node.set_pid(pid);
        }
        if let Some(tcp_listener) = tcp_listener {
            let address = (*tcp_listener.socket_address()).into();
            self.set_tcp_listener_address(&node.name(), &address)
                .await?;
            node = node.set_tcp_listener_address(address)
        }
        Ok(node)
    }

    /// Create a node, with some optional associated values:
    ///
    ///  - an identity name. That identity is used by the `NodeManager` to create secure channels
    ///  - a project name. It is used to create policies on resources provisioned on a node (like a TCP outlet for example)
    #[instrument(skip_all, fields(node_name = node_name, identity_name = identity_name.clone(), project_name = project_name.clone()))]
    pub async fn create_node_with_optional_values(
        &self,
        node_name: &str,
        identity_name: &Option<String>,
        project_name: &Option<String>,
    ) -> Result<NodeInfo> {
        let identity = match identity_name {
            Some(name) => self.get_named_identity(name).await?,
            None => self.get_or_create_default_named_identity().await?,
        };
        let node = self
            .create_node_with_identifier(node_name, &identity.identifier())
            .await?;
        self.set_node_project(node_name, project_name).await?;
        Ok(node)
    }

    /// This method creates a node with an associated identity
    /// The vault used to create the identity is the default vault
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn create_node(&self, node_name: &str) -> Result<NodeInfo> {
        let identity = self.create_identity_with_name(&random_name()).await?;
        self.create_node_with_identifier(node_name, &identity.identifier())
            .await
    }

    pub fn backup_logs(&self, node_name: &str) -> Result<()> {
        // Atm node dir only has logs
        let node_dir = self.node_dir(node_name);

        let now = now()?;

        let backup_dir = Self::backup_default_dir()?.join(format!("{}-{node_name}", now.0));
        std::fs::create_dir_all(&backup_dir)?;

        info!("Backing up logs for {node_name} from {node_dir:?} to {backup_dir:?}");

        // Move state to backup directory
        for entry in std::fs::read_dir(node_dir)? {
            let entry = entry?;
            let from = entry.path();
            let to = backup_dir.join(entry.file_name());

            std::fs::copy(from, to)?;
        }

        info!("Logs for {node_name} were backed up to {backup_dir:?}");
        Ok(())
    }

    /// Delete a node
    ///  - first stop it if it is running
    ///  - then remove it from persistent storage
    #[instrument(skip_all, fields(node_name = node_name, force = %force))]
    pub async fn delete_node(&self, node_name: &str, force: bool) -> Result<()> {
        self.stop_node(node_name, force).await?;
        self.remove_node(node_name).await?;
        Ok(())
    }

    /// Delete all created nodes
    #[instrument(skip_all, fields(force = %force))]
    pub async fn delete_all_nodes(&self, force: bool) -> Result<()> {
        let nodes = self.nodes_repository().get_nodes().await?;
        for node in nodes {
            self.delete_node(&node.name(), force).await?;
        }
        Ok(())
    }

    /// This method can be used to start a local node first
    /// then create a project, and associate it to the node
    #[instrument(skip_all, fields(node_name = node_name, project_name = project_name.clone()))]
    pub async fn set_node_project(
        &self,
        node_name: &str,
        project_name: &Option<String>,
    ) -> Result<()> {
        let project = match project_name {
            Some(name) => Some(self.projects().get_project_by_name(name).await?),
            None => self.projects().get_default_project().await.ok(),
        };

        if let Some(project) = project {
            self.nodes_repository()
                .set_node_project_name(node_name, project.name())
                .await?
        };
        Ok(())
    }

    /// Remove a node:
    ///
    ///  - remove it from the repository
    ///  - remove the node log files
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn remove_node(&self, node_name: &str) -> Result<()> {
        // don't try to remove a node on a non-existent database
        if !self.database_path().exists() {
            return Ok(());
        };

        // remove the node from the database
        let repository = self.nodes_repository();
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
        let _ = std::fs::remove_dir_all(self.node_dir(node_name));
        debug!(name=%node_name, "node deleted");
        Ok(())
    }

    /// Stop a background node
    ///
    ///  - if force is true, send a SIGKILL signal to the node process
    #[instrument(skip_all, fields(node_name = node_name, force = %force))]
    pub async fn stop_node(&self, node_name: &str, force: bool) -> Result<()> {
        let node = self.get_node(node_name).await?;
        self.nodes_repository().set_no_node_pid(node_name).await?;
        if let Some(pid) = node.pid() {
            // avoid killing the current process, return successfully instead.
            // this is useful when we need to stop all the nodes, for example
            // during a reset
            if pid == process::id() {
                return Ok(());
            }

            // kill process
            let pid = nix::unistd::Pid::from_raw(pid as i32);
            let kill_signal = if force {
                signal::Signal::SIGKILL
            } else {
                signal::Signal::SIGTERM
            };
            signal::kill(pid, kill_signal)
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
            debug!(name = %node.name(), %pid, "sent stop signal to node process");

            // wait until the node has fully stopped
            let mut attempts = 0;
            let max_attempts = 50; // 5 seconds max
            let timeout = std::time::Duration::from_millis(100);
            let mut sys = System::new();
            let pid = Pid::from_u32(pid.as_raw() as u32);
            loop {
                sys.refresh_processes();
                if sys.process(pid).is_none() {
                    info!(name = %node.name(), %pid, "node process exited");
                    break;
                }
                if attempts > max_attempts {
                    warn!(name = %node.name(), %pid, "node process did not exit");
                    break;
                }
                // notify the user that the node is stopping if it takes too long
                if attempts == 5 {
                    self.notify_progress(format!(
                        "Waiting for node {} to stop",
                        color_primary(node_name)
                    ));
                }
                attempts += 1;
                tokio::time::sleep(timeout).await;
            }
        }

        Ok(())
    }

    /// Set a node as the default node
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn set_default_node(&self, node_name: &str) -> Result<()> {
        Ok(self.nodes_repository().set_default_node(node_name).await?)
    }

    /// Set a TCP listener address on a node when the TCP listener has been started
    #[instrument(skip_all, fields(node_name = node_name, address = %address))]
    pub async fn set_tcp_listener_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()> {
        self.nodes_repository()
            .set_tcp_listener_address(node_name, address)
            .await?;
        Ok(())
    }

    #[instrument(skip_all, fields(node_name = node_name, address = %address))]
    pub async fn set_node_http_server_addr(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()> {
        Ok(self
            .nodes_repository()
            .set_http_server_address(node_name, address)
            .await?)
    }

    /// Specify that a node is an authority node
    /// This is used to display the node status since if the node TCP listener is not accessible
    /// without a secure channel
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn set_as_authority_node(&self, node_name: &str) -> Result<()> {
        Ok(self
            .nodes_repository()
            .set_as_authority_node(node_name)
            .await?)
    }

    /// Set the current process id on a background node
    /// Keeping track of a background node process id allows us to kill its process when stopping the node
    #[instrument(skip_all, fields(node_name = node_name, pid = %pid))]
    pub async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()> {
        Ok(self.nodes_repository().set_node_pid(node_name, pid).await?)
    }
}

/// The following methods return nodes data
impl CliState {
    /// Return a node by name
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn get_node(&self, node_name: &str) -> Result<NodeInfo> {
        if let Some(node) = self.nodes_repository().get_node(node_name).await? {
            Ok(node)
        } else {
            Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("There is no node with name {node_name}"),
            ))?
        }
    }

    /// Return all the created nodes
    #[instrument(skip_all)]
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        Ok(self.nodes_repository().get_nodes().await?)
    }

    /// Return information about the default node (if there is one)
    #[instrument(skip_all)]
    pub async fn get_default_node(&self) -> Result<NodeInfo> {
        Ok(self
            .nodes_repository()
            .get_default_node()
            .await?
            .ok_or(Error::new(
                Origin::Api,
                Kind::NotFound,
                "There is no default node",
            ))?)
    }

    /// Return the node information for the given node name, otherwise for the default node
    #[instrument(skip_all, fields(node_name = node_name.clone()))]
    pub async fn get_node_or_default(&self, node_name: &Option<String>) -> Result<NodeInfo> {
        match node_name {
            Some(name) => self.get_node(name).await,
            None => self.get_default_node().await,
        }
    }

    /// Return the project associated to a node if there is one
    #[instrument(skip_all, fields(node_name = node_name))]
    pub async fn get_node_project(&self, node_name: &str) -> Result<Project> {
        match self
            .nodes_repository()
            .get_node_project_name(node_name)
            .await?
        {
            Some(project_name) => self.projects().get_project_by_name(&project_name).await,
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no project associated to node {node_name}"),
            ))?,
        }
    }

    /// Return the stdout log file used by a node
    #[instrument(skip_all, fields(node_name = node_name))]
    pub fn stdout_logs(&self, node_name: &str) -> Result<PathBuf> {
        let node_dir = self.create_node_dir(node_name)?;
        let current_log_file = std::fs::read_dir(node_dir)?
            .flatten()
            .filter(|entry| {
                if let (Some(name), Ok(metadata)) = (entry.file_name().to_str(), entry.metadata()) {
                    name.contains("stdout") && metadata.is_file()
                } else {
                    false
                }
            })
            .max_by_key(|file| file.metadata().unwrap().modified().unwrap())
            .ok_or(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no log file for the node {node_name}"),
            ))?;
        Ok(current_log_file.path())
    }
}

/// Private functions
impl CliState {
    /// This method creates a node
    #[instrument(skip_all, fields(node_name = node_name, identifier = %identifier))]
    pub async fn create_node_with_identifier(
        &self,
        node_name: &str,
        identifier: &Identifier,
    ) -> Result<NodeInfo> {
        let repository = self.nodes_repository();

        let is_default = repository.is_default_node(node_name).await?
            || repository.get_nodes().await?.is_empty();

        let tcp_listener_address = repository.get_tcp_listener_address(node_name).await?;
        let http_server_address = repository.get_http_server_address(node_name).await?;

        let node_info = NodeInfo::new(
            node_name.to_string(),
            identifier.clone(),
            0,
            is_default,
            false,
            tcp_listener_address,
            Some(process::id()),
            http_server_address,
        );
        repository.store_node(&node_info).await?;
        Ok(node_info)
    }

    /// Return the nodes using a given identity
    #[instrument(skip_all, fields(identity_name = identity_name))]
    pub(super) async fn get_nodes_by_identity_name(
        &self,
        identity_name: &str,
    ) -> Result<Vec<NodeInfo>> {
        let identifier = self.get_identifier_by_name(identity_name).await?;
        Ok(self
            .nodes_repository()
            .get_nodes_by_identifier(&identifier)
            .await?)
    }

    /// Return the vault which was used to create the identity associated to a node
    #[instrument(skip_all, fields(node_name = node_name))]
    pub(super) async fn get_node_vault(&self, node_name: &str) -> Result<NamedVault> {
        let identifier = self.get_node(node_name).await?.identifier();
        let identity = self.get_named_identity_by_identifier(&identifier).await?;
        self.get_named_vault(&identity.vault_name()).await
    }

    /// Create a directory used to store files specific to a node
    fn create_node_dir(&self, node_name: &str) -> Result<PathBuf> {
        let path = self.node_dir(node_name);
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Return the default directory used by a node
    pub fn default_node_dir(node_name: &str) -> Result<PathBuf> {
        Ok(Self::make_node_dir_path(
            &CliState::default_dir()?,
            node_name,
        ))
    }

    /// Return the directory used by a node
    pub fn node_dir(&self, node_name: &str) -> PathBuf {
        Self::make_node_dir_path(&self.dir(), node_name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Decode, Encode)]
#[serde(rename_all = "lowercase", tag = "status", content = "pid")]
pub enum NodeProcessStatus {
    #[n(0)]
    Running(#[n(0)] u32),
    #[n(1)]
    Zombie(#[n(0)] u32),
    #[n(2)]
    Stopped,
}

impl NodeProcessStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, NodeProcessStatus::Running(_))
    }
}

impl Display for NodeProcessStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            NodeProcessStatus::Running(_) => ConnectionStatus::Up,
            NodeProcessStatus::Zombie(_) => ConnectionStatus::Down,
            NodeProcessStatus::Stopped => ConnectionStatus::Down,
        };
        let pid = match self {
            NodeProcessStatus::Running(pid) => Some(pid),
            NodeProcessStatus::Zombie(pid) => Some(pid),
            NodeProcessStatus::Stopped => None,
        };
        write!(f, "The node is {status}")?;
        if let Some(pid) = pid {
            write!(f, ", with PID {pid}")?;
        }
        Ok(())
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
    http_server_address: Option<InternetAddress>,
}

impl NodeInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        identifier: Identifier,
        verbosity: u8,
        is_default: bool,
        is_authority: bool,
        tcp_listener_address: Option<InternetAddress>,
        pid: Option<u32>,
        http_server_address: Option<InternetAddress>,
    ) -> Self {
        Self {
            name,
            identifier,
            verbosity,
            is_default,
            is_authority,
            tcp_listener_address,
            pid,
            http_server_address,
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

    pub fn http_server_address(&self) -> Option<InternetAddress> {
        self.http_server_address.clone()
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn set_pid(&self, pid: u32) -> NodeInfo {
        let mut result = self.clone();
        result.pid = Some(pid);
        result
    }

    pub fn set_tcp_listener_address(&self, address: InternetAddress) -> NodeInfo {
        let mut result = self.clone();
        result.tcp_listener_address = Some(address);
        result
    }

    /// Return true if there is a running process corresponding to the node process id
    pub fn is_running(&self) -> bool {
        matches!(self.status(), NodeProcessStatus::Running(_))
    }

    /// Return the status of the node process corresponding to the node process id
    pub fn status(&self) -> NodeProcessStatus {
        if let Some(pid) = self.pid() {
            let mut sys = System::new();
            sys.refresh_processes();
            if let Some(p) = sys.process(Pid::from(pid as usize)) {
                // Under certain circumstances the process can be in a state where it's not running
                // and we are unable to kill it. For example, `kill -9` a process created by
                // `node create` in a Docker environment will result in a zombie process.
                if matches!(p.status(), ProcessStatus::Dead | ProcessStatus::Zombie) {
                    NodeProcessStatus::Zombie(pid)
                } else {
                    NodeProcessStatus::Running(pid)
                }
            } else {
                NodeProcessStatus::Stopped
            }
        } else {
            NodeProcessStatus::Stopped
        }
    }

    pub fn route(&self) -> Result<MultiAddr> {
        let mut m = MultiAddr::default();
        m.push_back(Node::new(&self.name))?;
        Ok(m)
    }

    pub fn verbose_route(&self) -> Result<Option<MultiAddr>> {
        if let Some(port) = self.tcp_listener_port() {
            let mut m = MultiAddr::default();
            m.push_back(DnsAddr::new("localhost"))?;
            m.push_back(Tcp::new(port))?;
            Ok(Some(m))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cloud::project::models::ProjectModel;
    use crate::config::lookup::InternetAddress;
    use std::net::SocketAddr;
    use std::str::FromStr;

    use super::*;

    #[tokio::test]
    async fn test_create_node() -> Result<()> {
        let cli = CliState::test().await?;

        // a node can be created with just a name
        let node_name = "node-1";
        let result = cli.create_node(node_name).await?;
        assert_eq!(result.name(), node_name.to_string());

        // the first node is the default one
        let result = cli.get_default_node().await?.name();
        assert_eq!(result, node_name.to_string());

        // as a consequence, a default identity must have been created
        let result = cli.get_or_create_default_named_vault().await.ok();
        assert!(result.is_some());

        let result = cli.get_or_create_default_named_identity().await.ok();
        assert!(result.is_some());

        // that identity is associated to the node
        let identifier = result.unwrap().identifier();
        let result = cli.get_node(node_name).await?.identifier();
        assert_eq!(result, identifier);
        Ok(())
    }

    #[tokio::test]
    async fn test_update_node() -> Result<()> {
        let cli = CliState::test().await?;

        // create a node
        let node_name = "node-1";
        let _ = cli.create_node(node_name).await?;
        cli.set_tcp_listener_address(
            node_name,
            &SocketAddr::from_str("127.0.0.1:0").unwrap().into(),
        )
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
            !cli.node_dir(node1).exists(),
            "the node directory must be deleted"
        );

        // then node 2 should be the default node
        let result = cli.get_default_node().await?;
        assert_eq!(result, node_info2.set_as_default());
        Ok(())
    }

    #[tokio::test]
    async fn test_create_node_with_optional_values() -> Result<()> {
        let cli = CliState::test().await?;

        // a node can be created with just a name
        let node = cli
            .create_node_with_optional_values("node-1", &None, &None)
            .await?;
        let result = cli.get_node(&node.name()).await?;
        assert_eq!(result.name(), node.name());

        // a node can be created with a name and an existing identity
        let identity = cli.create_identity_with_name("name").await?;
        let node = cli
            .create_node_with_optional_values("node-2", &Some(identity.name()), &None)
            .await?;
        let result = cli.get_node(&node.name()).await?;
        assert_eq!(result.identifier(), identity.identifier());

        // a node can be created with a name, an existing identity and an existing project
        let project = ProjectModel {
            id: "project_id".to_string(),
            name: "project_name".to_string(),
            space_name: "1".to_string(),
            access_route: "".to_string(),
            users: vec![],
            space_id: "1".to_string(),
            identity: None,
            project_change_history: None,
            authority_access_route: None,
            authority_identity: None,
            okta_config: None,
            kafka_config: None,
            version: None,
            running: None,
            operation_id: None,
            user_roles: vec![],
        };
        cli.projects()
            .import_and_store_project(project.clone())
            .await?;

        let node = cli
            .create_node_with_optional_values(
                "node-4",
                &Some(identity.name()),
                &Some(project.name.clone()),
            )
            .await?;
        let result = cli.get_node_project(&node.name()).await?;
        assert_eq!(result.name(), &project.name);

        Ok(())
    }
}
