use ockam_core::async_trait;
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
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_identity::authenticated_storage::{
    AuthenticatedAttributeStorage, AuthenticatedStorage, IdentityAttributeStorage,
};
use ockam_identity::IdentitySecureChannelLocalInfo;

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
/// A similar access control policy is available as [`crate::policy::PolicyAccessControl`] where
/// as [`crate::PolicyStorage`] can be used to retrieve a specific policy for a given resource and action
pub struct AbacAccessControl<S> {
    attributes: S,
    expression: Expr,
    environment: Env,
}

/// Debug implementation printing out the policy expression only
impl<S> Debug for AbacAccessControl<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let expression = self.expression.clone();
        f.write_str(format!("{expression:?}").as_str())
    }
}

impl<S> AbacAccessControl<S> {
    /// Create a new AccessControl using a specific policy for checking attributes
    pub fn new(attributes: S, expression: Expr, environment: Env) -> Self {
        Self {
            attributes,
            expression,
            environment,
        }
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated attribute with the correct name and value
    pub fn create(
        storage: &S,
        attribute_name: &str,
        attribute_value: &str,
    ) -> AbacAccessControl<AuthenticatedAttributeStorage<S>>
    where
        S: AuthenticatedStorage + Clone,
    {
        let expression = List(vec![
            Ident("=".into()),
            Ident(format!("subject.{attribute_name}")),
            Str(attribute_value.into()),
        ]);
        AbacAccessControl::new(
            AuthenticatedAttributeStorage::new(storage.clone()),
            expression,
            Env::new(),
        )
    }
}

#[async_trait]
impl<S> IncomingAccessControl for AbacAccessControl<S>
where
    S: IdentityAttributeStorage,
{
    /// Return true if the sender of the message is validated by the expression stored in AbacAccessControl
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Get identity identifier from message metadata:
        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg.local_message()) {
            info.their_identity_id().clone()
        } else {
            log::debug! {
                policy = %self.expression,
                "identity identifier not found; access denied"
            }
            return Ok(false);
        };

        let mut environment = self.environment.clone();

        // Get identity attributes and populate the environment:
        if let Some(attrs) = self.attributes.get_attributes(&id).await? {
            for (key, value) in attrs.attrs() {
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
