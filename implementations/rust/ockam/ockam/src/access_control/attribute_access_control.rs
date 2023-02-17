use crate::alloc::string::ToString;
use ockam_core::async_trait;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::str;
use ockam_core::{IncomingAccessControl, RelayMessage};

use crate::authenticated_storage::{
    AuthenticatedAttributeStorage, AuthenticatedStorage, IdentityAttributeStorage,
};
use crate::identity::IdentitySecureChannelLocalInfo;
use crate::Result;
use ockam_abac::Expr::*;
use ockam_abac::{eval, Env, Expr};
use ockam_core::compat::boxed::Box;

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
pub struct AbacAccessControl<S> {
    attributes_storage: S,
    expression: Expr,
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
    pub fn new(attributes_storage: S, expression: Expr) -> Self {
        Self {
            attributes_storage,
            expression,
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
        // Get identity identifier from message metadata
        let their_identity_id =
            if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg.local_message()) {
                info.their_identity_id().clone()
            } else {
                return Ok(false);
            };

        // Get identity attributes and populate the environment
        let entry = if let Some(a) = self
            .attributes_storage
            .get_attributes(&their_identity_id)
            .await?
        {
            a
        } else {
            return Ok(false);
        };

        let mut e = Env::new();

        for (k, v) in entry.attrs() {
            if let Ok(s) = str::from_utf8(v) {
                e.put(format!("subject.{k}"), Str(s.to_string()));
            }
        }

        // Finally, evaluate the expression and return the result:
        match eval(&self.expression, &e) {
            Ok(Bool(b)) => Ok(b),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    }
}
