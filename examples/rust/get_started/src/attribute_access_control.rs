use ockam::abac::Expr::*;
use ockam::abac::{eval, Env, Expr};
use ockam::authenticated_storage::AuthenticatedStorage;
use ockam::identity::credential::AttributesStorageUtils;
use ockam::identity::IdentitySecureChannelLocalInfo;
use ockam::Result;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{IncomingAccessControl, RelayMessage};
use std::fmt::{Debug, Formatter};
use std::str;
use std::sync::Arc;

/// Create an IncomingAccessControl which will verify that the sender of
/// a message has an authenticated attribute with the correct name and value
pub fn create_attribute_access_control<S>(
    storage: S,
    attribute_name: &str,
    attribute_value: &str,
) -> Arc<dyn IncomingAccessControl>
where
    S: AuthenticatedStorage,
{
    let expression = List(vec![
        Ident("=".into()),
        Ident(format!("subject.{attribute_name}")),
        Str(attribute_value.into()),
    ]);
    Arc::new(AbacAccessControl::new(storage, expression))
}

/// This AccessControl uses a storage for authenticated attributes in order
/// to verify if a policy expression is valid
pub struct AbacAccessControl<S> {
    attributes_storage: S,
    expression: Expr,
}

impl<S> Debug for AbacAccessControl<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let expression = self.expression.clone();
        f.write_str(format!("{expression:?}").as_str())
    }
}

impl<S> AbacAccessControl<S> {
    fn new(attributes_storage: S, expression: Expr) -> Self {
        Self {
            attributes_storage,
            expression,
        }
    }
}

#[async_trait]
impl<S> IncomingAccessControl for AbacAccessControl<S>
where
    S: AuthenticatedStorage,
{
    /// Return true if the sender of the message is validated by the expression stored in AbacAccessControl
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Get identity identifier from message metadata:
        let their_identity_id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(msg.local_message()) {
            info.their_identity_id().clone()
        } else {
            return Ok(false);
        };

        // Get identity attributes and populate the environment:
        let attrs = if let Some(a) =
            AttributesStorageUtils::get_attributes(&their_identity_id, &self.attributes_storage).await?
        {
            a
        } else {
            return Ok(false);
        };

        let mut e = Env::new();

        for (k, v) in &attrs {
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
