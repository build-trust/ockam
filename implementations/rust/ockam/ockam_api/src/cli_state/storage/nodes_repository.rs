use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::Result;

use crate::cli_state::NodeInfo;
use crate::cloud::project::ProjectName;
use crate::config::lookup::InternetAddress;

/// This trait supports the storage of node data:
///
///  - a node has a unique name
///  - a node is always associated to an identifier
///  - a node can be associated to a (single) project
///  - when a node is running we can persist its process id and its TCP listener address
///  - one of the nodes is always set as the default node
///  - a node can be set as an authority node. The purpose of this flag is to be able to display
///    the node status without being able to start a TCP connection since the TCP listener might not be accessible
///
#[async_trait]
pub trait NodesRepository: Send + Sync + 'static {
    /// Store or update the information about a node
    async fn store_node(&self, node_info: &NodeInfo) -> Result<()>;

    /// Get the list of all the nodes
    async fn get_nodes(&self) -> Result<Vec<NodeInfo>>;

    /// Get a node by name
    async fn get_node(&self, node_name: &str) -> Result<Option<NodeInfo>>;

    /// Get the nodes using a given identifier
    async fn get_nodes_by_identifier(&self, identifier: &Identifier) -> Result<Vec<NodeInfo>>;

    /// Get the node set as default
    async fn get_default_node(&self) -> Result<Option<NodeInfo>>;

    /// Set a node set the default node
    async fn set_default_node(&self, node_name: &str) -> Result<()>;

    /// Return true if a node with the given name is the default node
    async fn is_default_node(&self, node_name: &str) -> Result<bool>;

    /// Delete a node given its name
    async fn delete_node(&self, node_name: &str) -> Result<()>;

    /// Set the TCP listener of a node
    async fn set_tcp_listener_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()>;

    /// Set that node as an authority node
    async fn set_as_authority_node(&self, node_name: &str) -> Result<()>;

    /// Get the TCP listener of a node
    async fn get_tcp_listener_address(&self, node_name: &str) -> Result<Option<InternetAddress>>;

    /// Set the process id of a node
    async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()>;

    /// Unset the process id of a node
    async fn set_no_node_pid(&self, node_name: &str) -> Result<()>;

    /// Associate a node to a project
    async fn set_node_project_name(&self, node_name: &str, project_name: ProjectName)
        -> Result<()>;

    /// Return the name of the project associated to a node
    async fn get_node_project_name(&self, node_name: &str) -> Result<Option<ProjectName>>;
}
