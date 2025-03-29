use crate::tokio::sync::mpsc::error::SendError;
use core::fmt;
use core::time::Duration;
use ockam_core::{
    compat::error::Error as StdError,
    errcode::{Kind, Origin},
    Address, Error,
};

/// Enumeration of error causes in ockam_node
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
pub enum NodeError {
    /// An address operation failed
    ///
    /// An address either refers to a Worker or a Processor
    Address(Address),
    /// A data retrieval operation failed
    Data,
    /// A failure occurred because of invalid node state
    NodeState(NodeReason),
    /// A failure occurred because of invalid worker state
    WorkerState(WorkerReason),
    /// A failure occurred because of invalid address router state
    RouterState(RouterReason),
}

impl NodeError {
    /// Turn a NodeError into a Kind::NotFound ockam_core::Error
    #[track_caller]
    pub fn not_found(self) -> Error {
        Error::new(Origin::Node, Kind::NotFound, self)
    }
    /// Turn a NodeError into a Kind::AlreadyExists ockam_core::Error
    #[track_caller]
    pub fn already_exists(self) -> Error {
        Error::new(Origin::Node, Kind::AlreadyExists, self)
    }
    /// Turn a NodeError into a Kind::Conflict ockam_core::Error
    #[track_caller]
    pub fn conflict(self) -> Error {
        Error::new(Origin::Node, Kind::Conflict, self)
    }
    /// Turn a NodeError into a Kind::Internal ockam_core::Error
    #[track_caller]
    pub fn internal(self) -> Error {
        Error::new(Origin::Node, Kind::Internal, self)
    }
    /// Create an ockam_core::Error based on a tokio::SendError
    #[track_caller]
    pub(crate) fn from_send_err<T: fmt::Debug>(err: SendError<T>) -> Error {
        Error::new(
            Origin::Node,
            Kind::Internal,
            NodeError::NodeState(NodeReason::Unknown),
        )
        .context("SendError", err)
    }

    /// Create an ockam_core::Error an elapsed timeout
    #[track_caller]
    pub(crate) fn with_timeout(self, duration: Duration) -> Error {
        Error::new(
            Origin::Node,
            Kind::Timeout,
            format!("timeout: {duration:?}"),
        )
        .context("Type", self)
    }
}

impl StdError for NodeError {}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Address(addr) => format!("operation failed for address {}", addr),
                Self::Data => "failed to load data".into(),
                Self::NodeState(reason) => format!("failed because node state: {}", reason),
                Self::WorkerState(reason) => format!("failed because worker state: {}", reason),
                Self::RouterState(reason) => format!("failed because router state: {}", reason),
            }
        )
    }
}

/// Reasons why adding an external router has failed
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum RouterReason {
    /// A duplicate router was registered
    Duplicate,
    /// The provided router address type is not valid
    InvalidAddrType,
    /// Empty Address Set
    EmptyAddressSet,
}

impl fmt::Display for RouterReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Duplicate => "a router for this type already exists",
                Self::InvalidAddrType => "you can not register router for this address type",
                Self::EmptyAddressSet => "address set cannot be empty",
            }
        )
    }
}

/// Reasons why a generic Ockam Node operation has failed
///
/// This includes many of the internal I/O failures
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum NodeReason {
    // TODO: currently we just tag all I/O events as "Unknown" but in
    // the future we should collect more information about WHY a
    // certain I/O operation failed.
    /// The node is in an unknown state
    Unknown,
    /// The node is shutting down
    Shutdown,
    /// The node has been corrupted
    Corrupt,
}

impl fmt::Display for NodeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Unknown => "unknown node state",
                Self::Shutdown => "ockam node is shutting down",
                Self::Corrupt => "ockam node is corrupt and can not be recovered",
            }
        )
    }
}

/// Reasons why a worker operation has failed
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum WorkerReason {
    /// The worker is shutting down
    Shutdown,
    /// The worker is faulty and waiting for supervisor commands
    Faulty,
    /// The worker is otherwise corrupt and can not be recovered
    Corrupt,
    /// Couldn't send shutdown signal
    CtrlChannelError,
}

impl fmt::Display for WorkerReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Shutdown => "target worker is shutting down",
                Self::Faulty => "target worker is faulty and waiting for supervisor",
                Self::Corrupt => "target worker is corrupt and can not be recovered",
                Self::CtrlChannelError => "target worker cannot receive shutdown signal",
            }
        )
    }
}
