use crate::{AbacAccessControl, Action, Env, Expr, PolicyStorage, Resource};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    async_trait, AllowAll, DenyAll, Error, LocalInfo, OutgoingAccessControl,
    OutgoingAccessControlFactory, RelayMessage,
};
use ockam_identity::{IdentitiesRepository, IdentityIdentifier, IdentitySecureChannelLocalInfo};

pub struct OutgoingPolicyFactory {
    policies: Arc<dyn PolicyStorage>,
    repository: Arc<dyn IdentitiesRepository>,
    resource: Resource,
    action: Action,
    environment: Env,
}

impl Debug for OutgoingPolicyFactory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let resource = &self.resource;
        let action = &self.resource;
        let environment = &self.environment;
        f.write_str(format!("resource {resource:?}").as_str())?;
        f.write_str(format!("action {action:?}").as_str())?;
        f.write_str(format!("environment {environment:?}").as_str())
    }
}

impl OutgoingPolicyFactory {
    pub fn new(
        policies: Arc<dyn PolicyStorage>,
        repository: Arc<dyn IdentitiesRepository>,
        resource: Resource,
        action: Action,
        environment: Env,
    ) -> Self {
        Self {
            policies,
            repository,
            resource,
            action,
            environment,
        }
    }
}

#[async_trait]
impl OutgoingAccessControlFactory for OutgoingPolicyFactory {
    async fn create(
        &self,
        local_info: &[LocalInfo],
    ) -> ockam_core::Result<Arc<dyn OutgoingAccessControl>> {
        let expr = if let Some(expr) = self
            .policies
            .get_policy(&self.resource, &self.action)
            .await?
        {
            if let Expr::Bool(b) = expr {
                return if b {
                    Ok(Arc::new(AllowAll))
                } else {
                    Ok(Arc::new(DenyAll))
                };
            } else {
                expr
            }
        } else {
            return Ok(Arc::new(DenyAll));
        };

        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info_from_list(local_info) {
            info.their_identity_id()
        } else {
            return Err(Error::new(
                Origin::Authorization,
                Kind::NotFound,
                "no identity found",
            ));
        };

        Ok(Arc::new(StaticPolicy {
            abac_access_control: AbacAccessControl::new(
                self.repository.clone(),
                expr,
                self.environment.clone(),
            ),
            their_identity_identifier: id,
        }))
    }
}

#[derive(Debug)]
struct StaticPolicy {
    abac_access_control: AbacAccessControl,
    their_identity_identifier: IdentityIdentifier,
}

#[async_trait]
impl OutgoingAccessControl for StaticPolicy {
    async fn is_authorized(&self, _message: &RelayMessage) -> ockam_core::Result<bool> {
        self.abac_access_control
            .is_identity_authorized(&self.their_identity_identifier)
            .await
    }
}
