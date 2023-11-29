use core::str::from_utf8;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::str;
use ockam_core::compat::vec::vec;
use ockam_core::Result;
use ockam_core::{IncomingAccessControl, RelayMessage};
use tracing as log;

use crate::expr::str;
use crate::Expr::*;
use crate::{eval, Env, Expr};
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_identity::{Identifier, IdentityAttributesRepository, IdentitySecureChannelLocalInfo};

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
/// A similar access control policy is available as [`crate::policy::PolicyAccessControl`] where
/// as [`crate::PoliciesRepository`] can be used to retrieve a specific policy for a given resource and action
pub struct AbacAccessControl {
    identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    expression: Expr,
    environment: Env,
}

/// Debug implementation printing out the policy expression only
impl Debug for AbacAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let expression = self.expression.clone();
        f.write_str(format!("{expression:?}").as_str())
    }
}

impl AbacAccessControl {
    /// Create a new AccessControl using a specific policy for checking attributes
    pub fn new(
        identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
        expression: Expr,
        environment: Env,
    ) -> Self {
        Self {
            identity_attributes_repository,
            expression,
            environment,
        }
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated attribute with the correct name and value
    pub fn create(
        identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
        attribute_name: &str,
        attribute_value: &str,
    ) -> AbacAccessControl
where {
        let expression = List(vec![
            Ident("=".into()),
            Ident(format!("subject.{attribute_name}")),
            Str(attribute_value.into()),
        ]);
        AbacAccessControl::new(identity_attributes_repository, expression, Env::new())
    }
}

impl AbacAccessControl {
    /// Returns true if the identity is authorized
    pub async fn is_identity_authorized(&self, id: Identifier) -> Result<bool> {
        let mut environment = self.environment.clone();

        // Get identity attributes and populate the environment:
        if let Some(attrs) = self
            .identity_attributes_repository
            .get_attributes(&id)
            .await?
        {
            for (key, value) in attrs.attrs() {
                let key = match from_utf8(key) {
                    Ok(key) => key,
                    Err(_) => {
                        log::warn! {
                            policy = %self.expression,
                            id     = %id,
                            "attribute key is not utf-8"
                        }
                        continue;
                    }
                };
                if key.find(|c: char| c.is_whitespace()).is_some() {
                    log::warn! {
                        policy = %self.expression,
                        id     = %id,
                        key    = %key,
                        "attribute key with whitespace ignored"
                    }
                }
                match str::from_utf8(value) {
                    Ok(s) => {
                        if environment.contains(key) {
                            log::debug! {
                                policy = %self.expression,
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
                            policy = %self.expression,
                            id     = %id,
                            key    = %key,
                            err    = %e,
                            "failed to interpret attribute as string"
                        }
                    }
                }
            }
        };

        // add the identifier itself as a subject parameter
        environment.put("subject.identifier", str(id.to_string()));

        // Finally, evaluate the expression and return the result:
        match eval(&self.expression, &environment) {
            Ok(Expr::Bool(b)) => {
                log::debug! {
                    policy        = %self.expression,
                    id            = %id,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(b)
            }
            Ok(x) => {
                log::warn! {
                    policy = %self.expression,
                    id     = %id,
                    expr   = %x,
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                log::warn! {
                    policy = %self.expression,
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
                policy = %self.expression,
                "identity identifier not found; access denied"
            }
            return Ok(false);
        };

        self.is_identity_authorized(id).await
    }
}
