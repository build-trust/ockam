use core::str::from_utf8;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::str;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::vec;
use ockam_core::identity::{SecureChannelLocalInfo, SecureChannelMetadata};
use ockam_core::{RelayMessage, Result};

use crate::expr::str;
use crate::{eval, Env, Expr};
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::transport::TransportLocalInfo;
use ockam_identity::{Identifier, IdentitiesAttributes};
use ockam_node::ContextRouter;
use tracing::{debug, warn};

/// Prefix we use to check for message attributes
pub const ABAC_MESSAGE_KEY: &str = "message";

/// Prefix we use to check if message is local
pub const ABAC_IS_LOCAL_KEY: &str = "is_local";

/// Prefix we use to check for subject attributes
pub const ABAC_SUBJECT_KEY: &str = "subject";

/// Prefix we use to check for message attributes
pub const ABAC_KEY: &str = "message";

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

pub struct OutgoingInfo {
    pub message_sent_to_the_same_node: bool,
    pub receiver_identifier: Option<Identifier>,
}

pub struct IncomingInfo {
    pub message_received_from_the_same_node: bool,
    pub sender_identifier: Option<Identifier>,
}

impl Abac {
    pub fn get_outgoing_info(
        ctx: &ContextRouter,
        relay_msg: &RelayMessage,
    ) -> Result<OutgoingInfo> {
        let mut outgoing_info = OutgoingInfo {
            message_sent_to_the_same_node: true,
            receiver_identifier: None,
        };

        match ctx.find_terminal_address(relay_msg.onward_route().iter())? {
            None => {}
            Some((_address, metadata)) => {
                outgoing_info.message_sent_to_the_same_node = false;

                if let Ok(metadata) =
                    SecureChannelMetadata::from_terminal_address_metadata(&metadata)
                {
                    outgoing_info.receiver_identifier = Some(metadata.their_identifier().into());
                }
            }
        }

        Ok(outgoing_info)
    }

    pub fn get_incoming_info(relay_msg: &RelayMessage) -> Result<IncomingInfo> {
        let mut incoming_info = IncomingInfo {
            message_received_from_the_same_node: true,
            sender_identifier: None,
        };

        if let Ok(_info) = TransportLocalInfo::find_info(relay_msg.local_message()) {
            incoming_info.message_received_from_the_same_node = false;
        }
        if let Ok(info) = SecureChannelLocalInfo::find_info(relay_msg.local_message()) {
            incoming_info.message_received_from_the_same_node = false;
            incoming_info.sender_identifier = Some(info.their_identifier().into());
        }

        Ok(incoming_info)
    }

    /// Returns true if the identity is authorized
    pub async fn is_authorized(
        &self,
        identifier: Option<&Identifier>,
        message_is_local: bool,
        expression: &Expr,
    ) -> Result<bool> {
        Self::is_authorized_static(
            self.identities_attributes.clone(),
            &self.environment,
            self.authority.as_ref(),
            identifier,
            message_is_local,
            expression,
        )
        .await
    }

    /// Returns true if the identity is authorized
    pub async fn is_authorized_static(
        identities_attributes: Arc<IdentitiesAttributes>,
        environment: &Env,
        authority: Option<&Identifier>,
        identifier: Option<&Identifier>,
        message_is_local: bool,
        expression: &Expr,
    ) -> Result<bool> {
        let mut environment = environment.clone();

        if message_is_local {
            environment.put(message_is_local_attribute().to_string(), Expr::CONST_TRUE);
        }

        if let Some(identifier) = identifier {
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
                                        environment.put(
                                            format!("{}.{key}", ABAC_SUBJECT_KEY),
                                            str(s.to_string()),
                                        );
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
        }

        // Finally, evaluate the expression and return the result:
        match eval(expression, &environment) {
            Ok(Expr::Bool(b)) => {
                debug! {
                    policy        = %expression,
                    id            = ?identifier,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(b)
            }
            Ok(x) => {
                warn! {
                    policy = %expression,
                    id     = ?identifier,
                    expr   = %x,
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                warn! {
                    policy = %expression,
                    id     = ?identifier,
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
    Expr::Ident(format!("{}.{}", ABAC_SUBJECT_KEY, ABAC_HAS_CREDENTIAL_KEY))
}

/// Identifier for the subject 'identifier' attribute
pub fn subject_identifier_attribute() -> Expr {
    Expr::Ident(format!("{}.{}", ABAC_SUBJECT_KEY, ABAC_IDENTIFIER_KEY))
}

/// Return a policy expression checking if the message is local
pub fn message_is_local_policy_expression() -> Expr {
    Expr::List(vec![
        Expr::Ident("=".to_string()),
        message_is_local_attribute(),
        Expr::Bool(true),
    ])
}

/// Identifier for the message 'is local' attribute
pub fn message_is_local_attribute() -> Expr {
    Expr::Ident(format!("{}.{}", ABAC_MESSAGE_KEY, ABAC_IS_LOCAL_KEY))
}
