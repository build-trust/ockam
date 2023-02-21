use core::{fmt, str};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::{async_trait, RelayMessage};
use ockam_core::{IncomingAccessControl, Result};
use ockam_identity::{
    authenticated_storage::IdentityAttributeStorage, IdentitySecureChannelLocalInfo,
};
use tracing as log;

use crate::eval::eval;
use crate::expr::str;
use crate::traits::PolicyStorage;
use crate::types::{Action, Resource};
use crate::{Env, Expr};

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
#[derive(Debug)]
pub struct PolicyAccessControl<P, S> {
    resource: Resource,
    action: Action,
    policies: P,
    attributes: S,
    environment: Env,
    overwrite: bool,
}

impl<P, S> PolicyAccessControl<P, S> {
    /// Create a new `PolicyAccessControl`.
    ///
    /// The policy expression is evaluated by getting subject attributes from
    /// the given authenticated storage, adding them the given environment,
    /// which may already contain other resource, action or subject attributes.
    pub fn new(policies: P, store: S, r: Resource, a: Action, env: Env) -> Self {
        Self {
            resource: r,
            action: a,
            policies,
            attributes: store,
            environment: env,
            overwrite: false,
        }
    }

    pub fn overwrite(&mut self) {
        self.overwrite = true
    }
}

#[async_trait]
impl<P, S> IncomingAccessControl for PolicyAccessControl<P, S>
where
    S: IdentityAttributeStorage + fmt::Debug,
    P: PolicyStorage + fmt::Debug,
{
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Load the policy expression for resource and action:
        let expr = if let Some(expr) = self
            .policies
            .get_policy(&self.resource, &self.action)
            .await?
        {
            if let Expr::Bool(b) = expr {
                // If the policy is a constant there is no need to populate
                // the environment or look for message metadata.
                return Ok(b);
            } else {
                expr
            }
        } else {
            // If no policy exists for this resource and action access is denied:
            log::debug! {
                resource = %self.resource,
                action   = %self.action,
                "no policy found; access denied"
            }
            return Ok(false);
        };

        // Get identity identifier from message metadata:
        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg.local_message()) {
            info.their_identity_id().clone()
        } else {
            log::debug! {
                resource = %self.resource,
                action   = %self.action,
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
                        resource = %self.resource,
                        action   = %self.action,
                        id       = %id,
                        key      = %key,
                        "attribute key with whitespace ignored"
                    }
                }
                match str::from_utf8(value) {
                    Ok(s) => {
                        if !self.overwrite && environment.contains(key) {
                            log::debug! {
                                resource = %self.resource,
                                action   = %self.action,
                                id       = %id,
                                key      = %key,
                                "attribute already present"
                            }
                            continue;
                        }
                        environment.put(format!("subject.{key}"), str(s.to_string()));
                    }
                    Err(e) => {
                        log::warn! {
                            resource = %self.resource,
                            action   = %self.action,
                            id       = %id,
                            key      = %key,
                            err      = %e,
                            "failed to interpret attribute as string"
                        }
                    }
                }
            }
        };

        //add the identifier itself as a subject parameter
        environment.put("subject.identifier", str(id.to_string()));

        // Finally, evaluate the expression and return the result:
        match eval(&expr, &environment) {
            Ok(Expr::Bool(b)) => {
                log::debug! {
                    resource      = %self.resource,
                    action        = %self.action,
                    id            = %id,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(b)
            }
            Ok(x) => {
                log::warn! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    expr     = %x,
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                log::warn! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    err      = %e,
                    "policy evaluation failed"
                }
                Ok(false)
            }
        }
    }
}
