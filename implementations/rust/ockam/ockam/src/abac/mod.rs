//! Attribute Based Access Control

pub mod access_control;
pub mod error;
pub mod mem;

mod local_info;
mod metadata;
mod types;
mod workers;

pub use local_info::*;
pub use metadata::*;
pub use types::*;
pub use workers::*;

use ockam_core::async_trait;
use ockam_core::Result;

/// The `Abac` trait provides an interface for implementing custom
/// ABAC backends.
///
/// An ABAC backend performs the following functions:
///
/// 1. provide an interface to the storage and retrieval of ABAC
///    attributes and authorization policies
/// 2. perform an authorization check on a given [`Subject`],
///    [`Resource`], [`Action`] triple
///
/// The ABAC logical schema has the form:
///
/// * Attributes: [`Subject`]  ||--|{ [`Attribute`] ||--|| (`key`, [`Value`])
/// * Policies: [`Resource`] ||--|{ `policy` ||--|| ([`Action`], [`Conditional`])
///
/// ([schema syntax](https://mermaid-js.github.io/mermaid/#/entityRelationshipDiagram?id=syntax))
#[async_trait]
pub trait Abac {
    /// Set the attributes for a given ABAC [`Subject`].
    ///
    /// Any pre-existing attribute entries for the subject will be
    /// replaced.
    async fn set_subject<I>(&self, s: Subject, attributes: I) -> Result<()>
    where
        I: IntoIterator<Item = Attribute> + Send + 'static;

    /// Set a [`Conditional`] policy entry for a given ABAC
    /// [`Resource`] and [`Action`] .
    ///
    /// Any pre-existing [`Action`] entries associated with the
    /// [`Resource`] will be replaced.
    async fn set_policy(&self, r: Resource, a: Action, c: &Conditional) -> Result<()>;

    /// Delete all attributes for the given ABAC [`Subject`].
    async fn del_subject(&self, s: &Subject) -> Result<()>;

    /// Delete all policy entries associated with the given ABAC
    /// [`Resource`].  [`Resource`].
    async fn del_policy(&self, r: &Resource) -> Result<()>;

    /// Perform an authorization check for the given ABAC [`Subject`],
    /// [`Resource`] and [`Action`].
    async fn is_authorized(&self, s: &Subject, r: &Resource, a: &Action) -> Result<bool>;
}
