use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::str;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::vec;
use ockam_core::Result;
use ockam_core::{IncomingAccessControl, RelayMessage};

use crate::abac::Abac;
use crate::abac::SUBJECT_KEY;
use crate::Expr::*;
use crate::{Env, Expr};
use ockam_core::compat::format;
use ockam_identity::{Identifier, IdentitiesAttributes};
use tracing::debug;

#[derive(Debug)]
pub struct IncomingAbac {
    expression: Expr,
    abac: Abac,
}

impl IncomingAbac {
    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated attribute that resolves the expression to `true`
    pub fn create(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Option<Identifier>,
        expression: Expr,
    ) -> Self {
        let abac = Abac::new(identities_attributes, authority, Env::new());

        Self { expression, abac }
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated attribute with the correct name and value
    pub fn create_name_value(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Option<Identifier>,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Self {
        let expression = List(vec![
            Ident("=".into()),
            Ident(format!("{SUBJECT_KEY}.{attribute_name}")),
            Str(attribute_value.into()),
        ]);
        Self::create(identities_attributes, authority, expression)
    }

    /// Create an AccessControl which will verify that the sender of
    /// a message has an authenticated credential without checking any attributes
    pub fn check_credential_only(
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
    ) -> Self {
        Self::create(identities_attributes, Some(authority), true.into())
    }

    /// Returns true if the sender of the message is validated by the expression stored in AbacAccessControl
    pub async fn is_authorized_impl(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let identifier = match Abac::get_incoming_identifier(relay_msg) {
            Some(identifier) => identifier,
            None => {
                debug! {
                    policy = %self.expression,
                    "identity identifier not found; access denied"
                }

                return Ok(false);
            }
        };

        self.abac
            .is_identity_authorized(&identifier, &self.expression)
            .await
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }

    pub fn abac(&self) -> &Abac {
        &self.abac
    }
}

#[async_trait]
impl IncomingAccessControl for IncomingAbac {
    /// Returns true if the sender of the message is validated by the expression stored in AbacAccessControl
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        self.is_authorized_impl(msg).await
    }
}
