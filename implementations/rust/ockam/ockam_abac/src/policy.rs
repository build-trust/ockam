use core::{fmt, str};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::{AccessControl, LocalMessage, Result};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{credential::AttributesStorageUtils, IdentitySecureChannelLocalInfo};
use tracing as log;

use crate::eval::eval;
use crate::expr::str;
use crate::{Env, Expr};

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
#[derive(Debug)]
pub struct PolicyAccessControl<S> {
    expression: Expr,
    attributes: S,
    environment: Env,
    overwrite: bool,
}

impl<S> PolicyAccessControl<S> {
    /// Create a new `PolicyAccessControl`.
    ///
    /// The policy expression is evaluated by getting subject attributes from
    /// the given authenticated storage, adding them the given envionment,
    /// which may already contain other resource, action or subject attributes.
    pub fn new(policy: Expr, store: S, env: Env) -> Self {
        Self {
            expression: policy,
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
impl<S> AccessControl for PolicyAccessControl<S>
where
    S: AuthenticatedStorage + fmt::Debug,
{
    async fn is_authorized(&self, msg: &LocalMessage) -> Result<bool> {
        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg) {
            info.their_identity_id().clone()
        } else {
            return Ok(false);
        };

        let attrs =
            if let Some(a) = AttributesStorageUtils::get_attributes(&id, &self.attributes).await? {
                a
            } else {
                return Ok(false);
            };

        let mut e = self.environment.clone();

        for (k, v) in &attrs {
            if k.find(|c: char| c.is_whitespace()).is_some() {
                log::warn!(%id, key = %k, "attribute key with whitespace ignored")
            }
            match str::from_utf8(v) {
                Ok(s) => {
                    if !self.overwrite && e.contains(k) {
                        log::debug!(%id, key = %k, "attribute already present");
                        continue;
                    }
                    e.put(format!("subject.{k}"), str(s.to_string()));
                }
                Err(e) => {
                    log::warn!(%id, err = %e, key = %k, "failed to interpret attribute as string")
                }
            }
        }

        match eval(&self.expression, &e) {
            Ok(Expr::Bool(b)) => {
                log::debug!(%id, is_authorized = %b, "policy evaluated");
                Ok(b)
            }
            Ok(x) => {
                log::warn!(%id, expr = %x, "evaluation did not yield a boolean result");
                Ok(false)
            }
            Err(e) => {
                log::warn!(%id, err = %e, "policy evaluation failed");
                Ok(false)
            }
        }
    }
}
