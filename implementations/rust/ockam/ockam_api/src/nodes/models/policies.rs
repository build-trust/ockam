use minicbor::{Decode, Encode};
use ockam_abac::{Action, Expr, ResourceName, ResourcePolicy, ResourceType, ResourceTypePolicy};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SetPolicyRequest {
    #[n(1)] pub resource: ResourceTypeOrName,
    #[n(2)] pub expression: Expr,
}

impl SetPolicyRequest {
    pub fn new(resource: ResourceTypeOrName, expression: Expr) -> Self {
        Self {
            resource,
            expression,
        }
    }
}

#[derive(Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PoliciesList {
    #[n(1)] resource_policies: Vec<ResourcePolicy>,
    #[n(2)] resource_type_policies: Vec<ResourceTypePolicy>,
}

impl PoliciesList {
    pub fn new(
        resource_policies: Vec<ResourcePolicy>,
        resource_type_policies: Vec<ResourceTypePolicy>,
    ) -> Self {
        Self {
            resource_policies,
            resource_type_policies,
        }
    }

    pub fn resource_policies(&self) -> &[ResourcePolicy] {
        &self.resource_policies
    }

    pub fn resource_type_policies(&self) -> &[ResourceTypePolicy] {
        &self.resource_type_policies
    }
}

/// A view for the specific policy types returned by policies repositories. This is used
/// to simplify the type returned by the NodeManager in the api requests.
#[derive(Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Policy {
    #[n(1)] resource: ResourceTypeOrName,
    #[n(2)] action: Action,
    #[n(3)] expression: Expr,
}

impl Policy {
    pub fn resource(&self) -> &ResourceTypeOrName {
        &self.resource
    }

    pub fn action(&self) -> &Action {
        &self.action
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }
}

impl From<ResourceTypePolicy> for Policy {
    fn from(policy: ResourceTypePolicy) -> Self {
        Policy {
            resource: ResourceTypeOrName::Type(policy.resource_type),
            action: policy.action,
            expression: policy.expression,
        }
    }
}

impl From<ResourcePolicy> for Policy {
    fn from(policy: ResourcePolicy) -> Self {
        Policy {
            resource: ResourceTypeOrName::Name(policy.resource_name),
            action: policy.action,
            expression: policy.expression,
        }
    }
}

/// A high-level representation of a resource distinguishing between resource types,
/// which are predefined and have a special meaning, and resource names, which are
/// user-defined and can be anything.
///
/// This type is used at the top level of the NodeManager to reduce the number of endpoints.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
pub enum ResourceTypeOrName {
    #[n(1)] Type(#[n(1)] ResourceType),
    #[n(2)] Name(#[n(1)] ResourceName),
}

impl ResourceTypeOrName {
    pub fn new(
        resource_type: Option<&ResourceType>,
        resource_name: Option<&ResourceName>,
    ) -> ockam_core::Result<Self> {
        Ok(match (resource_type, resource_name) {
            (Some(resource_type), _) => Self::Type(resource_type.clone()),
            (_, Some(resource_name)) => Self::Name(resource_name.clone()),
            _ => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Misuse,
                    "Resource or resource type must be provided",
                ))
            }
        })
    }
}

impl Display for ResourceTypeOrName {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let as_str = match self {
            Self::Type(g) => g.to_string(),
            Self::Name(s) => s.to_string(),
        };
        write!(f, "{}", as_str)
    }
}
