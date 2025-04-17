use core::fmt::Formatter;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::str;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::vec;
use ockam_core::RelayMessage;
use ockam_core::Result;
use ockam_core::{async_trait, OutgoingAccessControl};

use crate::abac::Abac;
use crate::abac::ABAC_SUBJECT_KEY;
use crate::Expr::*;
use crate::{Env, Expr};
use ockam_core::compat::format;
use ockam_identity::{Identifier, IdentitiesAttributes};
use ockam_node::ContextRouter;

pub struct OutgoingAbac {
    ctx: ContextRouter,
    expression: Expr,
    abac: Abac,
}

impl Debug for OutgoingAbac {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OutgoingAbac")
            .field("expression", &self.expression)
            .field("abac", &self.abac)
            .finish()
    }
}

impl OutgoingAbac {
    /// Create an AccessControl which will verify that the receiver of
    /// a message has an authenticated attribute that resolves the expression to `true`
    pub fn create(
        ctx: ContextRouter,
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Option<Identifier>,
        expression: Expr,
    ) -> Result<Self> {
        let abac = Abac::new(identities_attributes, authority, Env::new());

        Ok(Self {
            ctx,
            expression,
            abac,
        })
    }

    /// Create an AccessControl which will verify that the receiver of
    /// a message has an authenticated attribute with the correct name and value
    pub fn create_name_value(
        ctx: ContextRouter,
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Option<Identifier>,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<Self> {
        let expression = List(vec![
            Ident("=".into()),
            Ident(format!("{ABAC_SUBJECT_KEY}.{attribute_name}")),
            Str(attribute_value.into()),
        ]);
        Self::create(ctx, identities_attributes, authority, expression)
    }

    /// Create an AccessControl which will verify that the receiver of
    /// a message has an authenticated credential without checking any attributes
    pub fn check_credential_only(
        ctx: ContextRouter,
        identities_attributes: Arc<IdentitiesAttributes>,
        authority: Identifier,
    ) -> Result<Self> {
        Self::create(ctx, identities_attributes, Some(authority), true.into())
    }

    /// Returns true if the sender of the message is validated by the expression stored in AbacAccessControl
    pub async fn is_authorized_impl(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let outgoing_info = Abac::get_outgoing_info(&self.ctx, relay_msg)?;

        self.abac
            .is_authorized(
                outgoing_info.receiver_identifier.as_ref(),
                outgoing_info.message_sent_to_the_same_node,
                &self.expression,
            )
            .await
    }
}

#[async_trait]
impl OutgoingAccessControl for OutgoingAbac {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        self.is_authorized_impl(relay_msg).await
    }
}
