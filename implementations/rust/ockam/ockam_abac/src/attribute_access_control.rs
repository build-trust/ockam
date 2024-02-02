use core::str::from_utf8;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::str;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::vec;
use ockam_core::Result;
use ockam_core::{IncomingAccessControl, RelayMessage};

use crate::expr::str;
use crate::Expr::*;
use crate::{eval, Env, Expr, Policy};
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_identity::{Identifier, IdentitiesAttributes, IdentitySecureChannelLocalInfo};
use tracing as log;

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
/// A similar access control policy is available as [`crate::policy::PolicyAccessControl`] where
/// as [`crate::PoliciesRepository`] can be used to retrieve a specific policy for a given resource and action
pub struct AbacAccessControl {
    identities_attributes: Arc<IdentitiesAttributes>,
    authority: Identifier,
    policy: Policy,
    environment: Env,
}

/// Debug implementation printing out the policy expression only
impl Debug for AbacAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let expression = self.policy.expression().clone();
        f.write_str(format!("{expression:?}").as_str())
    }
}

impl AbacAccessControl {
    /// Create a new AccessControl using a specific policy for checking attributes
    pub fn new(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
        policy: Policy,
        environment: Env,
    ) -> Self {
        Self {
            identities_attributes,
            authority,
            policy,
            environment,
        }
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated attribute with the correct name and value
    pub fn create(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
        attribute_name: &str,
        attribute_value: &str,
    ) -> AbacAccessControl {
        let expression = List(vec![
            Ident("=".into()),
            Ident(format!("subject.{attribute_name}")),
            Str(attribute_value.into()),
        ]);
        AbacAccessControl::new(
            identities_attributes,
            authority,
            Policy::new(expression),
            Env::new(),
        )
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated credential without checking any attributes
    pub fn check_credential_only(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
    ) -> AbacAccessControl {
        AbacAccessControl::new(
            identities_attributes,
            authority,
            Policy::new(true.into()),
            Env::new(),
        )
    }
}

impl AbacAccessControl {
    /// Returns true if the identity is authorized
    pub async fn is_identity_authorized(&self, id: Identifier) -> Result<bool> {
        let mut environment = self.environment.clone();

        // Get identity attributes and populate the environment:
        if let Some(attrs) = self
            .identities_attributes
            .get_attributes(&id, &self.authority)
            .await?
        {
            for (key, value) in attrs.attrs() {
                let key = match from_utf8(key) {
                    Ok(key) => key,
                    Err(_) => {
                        log::warn! {
                            policy = %self.policy,
                            id     = %id,
                            "attribute key is not utf-8"
                        }
                        continue;
                    }
                };
                if key.find(|c: char| c.is_whitespace()).is_some() {
                    log::warn! {
                        policy = %self.policy,
                        id     = %id,
                        key    = %key,
                        "attribute key with whitespace ignored"
                    }
                }
                match str::from_utf8(value) {
                    Ok(s) => {
                        if environment.contains(key) {
                            log::debug! {
                                policy = %self.policy,
                                id     = %id,
                                key    = %key,
                                "attribute already present"
                            }
                        } else {
                            environment.put(format!("subject.{key}"), str(s.to_string()));
                        }
                    }
                    Err(e) => {
                        log::warn! {
                            policy = %self.policy,
                            id     = %id,
                            key    = %key,
                            err    = %e,
                            "failed to interpret attribute as string"
                        }
                    }
                }
            }
        }

        // add the identifier itself as a subject parameter
        environment.put("subject.identifier", str(id.to_string()));

        // Finally, evaluate the expression and return the result:
        match eval(self.policy.expression(), &environment) {
            Ok(Expr::Bool(b)) => {
                log::debug! {
                    policy        = %self.policy,
                    id            = %id,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(b)
            }
            Ok(x) => {
                log::warn! {
                    policy = %self.policy,
                    id     = %id,
                    expr   = %x,
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                log::warn! {
                    policy = %self.policy,
                    id     = %id,
                    err    = %e,
                    "policy evaluation failed"
                }
                Ok(false)
            }
        }
    }
}

#[async_trait]
impl IncomingAccessControl for AbacAccessControl {
    /// Returns true if the sender of the message is validated by the expression stored in AbacAccessControl
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Get identity identifier from message metadata:
        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg.local_message()) {
            info.their_identity_id()
        } else {
            log::debug! {
                policy = %self.policy,
                "identity identifier not found; access denied"
            }
            return Ok(false);
        };

        self.is_identity_authorized(id).await
    }
}
