use core::str::from_utf8;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::str;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::vec;
use ockam_core::{RelayMessage, SecureChannelMetadata};
use ockam_core::{Result, SecureChannelLocalInfo};

use crate::expr::str;
use crate::{eval, Env, Expr};
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_identity::{Identifier, IdentitiesAttributes};
use ockam_node::Context;
use tracing::{debug, warn};

/// Prefix we use to check for subject attributes
pub const SUBJECT_KEY: &str = "subject";

/// Key we use to indicate a subject has valid credential
pub const ABAC_HAS_CREDENTIAL_KEY: &str = "has_credential";

/// Key we use to check Identifier
pub const ABAC_IDENTIFIER_KEY: &str = "identifier";

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
/// A similar access control policy is available as [`crate::policy::PolicyAccessControl`] where
/// as [`crate::Policies`] can be used to retrieve a specific policy for a given resource and action
#[derive(Clone)]
pub struct Abac {
    identities_attributes: Arc<IdentitiesAttributes>,
    authority: Option<Identifier>,
    environment: Env,
}

/// Debug implementation printing out the policy expression only
impl Debug for Abac {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Authority: {:?}", self.authority)
    }
}

impl Abac {
    /// Create a new AccessControl using a specific policy for checking attributes
    pub fn new(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Option<Identifier>,
        environment: Env,
    ) -> Self {
        Self {
            identities_attributes,
            authority,
            environment,
        }
    }
}

impl Abac {
    pub async fn get_outgoing_identifier(
        ctx: &Context,
        relay_msg: &RelayMessage,
    ) -> Result<Option<Identifier>> {
        let terminal = if let Some(terminal) = ctx
            .find_terminal_address(relay_msg.onward_route().clone())
            .await?
        {
            terminal
        } else {
            return Ok(None);
        };

        if let Ok(metadata) = SecureChannelMetadata::from_terminal_address(&terminal) {
            Ok(Some(metadata.their_identifier().into()))
        } else {
            Ok(None)
        }
    }

    pub fn get_incoming_identifier(relay_msg: &RelayMessage) -> Option<Identifier> {
        let identifier =
            if let Ok(info) = SecureChannelLocalInfo::find_info(relay_msg.local_message()) {
                info.their_identifier()
            } else {
                return None;
            };

        Some(identifier.into())
    }

    /// Returns true if the identity is authorized
    pub async fn is_identity_authorized(
        &self,
        identifier: &Identifier,
        expression: &Expr,
    ) -> Result<bool> {
        Self::is_identity_authorized_static(
            self.identities_attributes.clone(),
            &self.environment,
            self.authority.as_ref(),
            identifier,
            expression,
        )
        .await
    }

    /// Returns true if the identity is authorized
    pub async fn is_identity_authorized_static(
        identities_attributes: Arc<IdentitiesAttributes>,
        environment: &Env,
        authority: Option<&Identifier>,
        identifier: &Identifier,
        expression: &Expr,
    ) -> Result<bool> {
        let mut environment = environment.clone();

        // add the identifier itself as a subject parameter
        // it's important to do it before we put other attributes, so it can't be overwritten
        environment.put(
            subject_identifier_attribute().to_string(),
            str(identifier.to_string()),
        );

        // Get identity attributes and populate the environment:
        if let Some(authority) = authority {
            match identities_attributes
                .get_attributes(identifier, authority)
                .await?
            {
                Some(attrs) => {
                    environment.put(
                        subject_has_credential_attribute().to_string(),
                        Expr::CONST_TRUE,
                    );

                    for (key, value) in attrs.attrs() {
                        let key = match from_utf8(key) {
                            Ok(key) => key,
                            Err(_) => {
                                warn! {
                                    policy = %expression,
                                    id     = %identifier,
                                    "attribute key is not utf-8"
                                }
                                continue;
                            }
                        };
                        if key.find(|c: char| c.is_whitespace()).is_some() {
                            warn! {
                                policy = %expression,
                                id     = %identifier,
                                key    = %key,
                                "attribute key with whitespace ignored"
                            }
                        }
                        match str::from_utf8(value) {
                            Ok(s) => {
                                if environment.contains(key) {
                                    warn! {
                                        policy = %expression,
                                        id     = %identifier,
                                        key    = %key,
                                        "attribute already present"
                                    }
                                } else {
                                    environment
                                        .put(format!("{}.{key}", SUBJECT_KEY), str(s.to_string()));
                                }
                            }
                            Err(e) => {
                                warn! {
                                    policy = %expression,
                                    id     = %identifier,
                                    key    = %key,
                                    err    = %e,
                                    "failed to interpret attribute as string"
                                }
                            }
                        }
                    }
                }
                None => {
                    environment.put(
                        subject_has_credential_attribute().to_string(),
                        Expr::CONST_FALSE,
                    );
                }
            }
        }

        // Finally, evaluate the expression and return the result:
        match eval(expression, &environment) {
            Ok(Expr::Bool(b)) => {
                debug! {
                    policy        = %expression,
                    id            = %identifier,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(b)
            }
            Ok(x) => {
                warn! {
                    policy = %expression,
                    id     = %identifier,
                    expr   = %x,
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                warn! {
                    policy = %expression,
                    id     = %identifier,
                    err    = %e,
                    env    = %environment,
                    "policy evaluation failed"
                }
                Ok(false)
            }
        }
    }
}

/// Return a policy expression checking if the subject has a valid credential
pub fn subject_has_credential_policy_expression() -> Expr {
    Expr::List(vec![
        Expr::Ident("=".to_string()),
        subject_has_credential_attribute(),
        Expr::Bool(true),
    ])
}

/// Identifier for the subject 'has_credential' attribute
pub fn subject_has_credential_attribute() -> Expr {
    Expr::Ident(format!("{}.{}", SUBJECT_KEY, ABAC_HAS_CREDENTIAL_KEY))
}

/// Identifier for the subject 'identifier' attribute
pub fn subject_identifier_attribute() -> Expr {
    Expr::Ident(format!("{}.{}", SUBJECT_KEY, ABAC_IDENTIFIER_KEY))
}
