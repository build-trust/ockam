use crate::policy::ResourceTypePolicy;
use crate::{
    subject_has_credential_policy_expression, Action, Env, Expr, PolicyAccessControl, Resource,
    ResourceName, ResourcePoliciesRepository, ResourcePolicy, ResourceType,
    ResourceTypePoliciesRepository,
};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_identity::{Identifier, IdentitiesAttributes};
use strum::IntoEnumIterator;
use tracing::debug;

#[derive(Clone)]
pub struct Policies {
    resources_policies_repository: Arc<dyn ResourcePoliciesRepository>,
    resource_types_policies_repository: Arc<dyn ResourceTypePoliciesRepository>,
}

impl Policies {
    pub fn new(
        resources_policies_repository: Arc<dyn ResourcePoliciesRepository>,
        resource_types_policies_repository: Arc<dyn ResourceTypePoliciesRepository>,
    ) -> Self {
        Self {
            resources_policies_repository,
            resource_types_policies_repository,
        }
    }

    //TODO #[instrument(skip_all, fields(resource = %resource, action = %action, env = %env, authority = %authority))]
    pub fn make_policy_access_control(
        &self,
        identities_attributes: Arc<IdentitiesAttributes>,
        resource: Resource,
        action: Action,
        env: Env,
        authority: Option<Identifier>,
    ) -> PolicyAccessControl {
        debug!(
            "set a policy access control for resource '{}' of type '{}' and action '{}'",
            &resource.resource_name, &resource.resource_type, &action
        );
        PolicyAccessControl::new(
            self.clone(),
            identities_attributes,
            authority,
            env,
            resource,
            action.clone(),
        )
    }

    pub async fn get_policies(&self) -> Result<(Vec<ResourcePolicy>, Vec<ResourceTypePolicy>)> {
        let resource_policies = self.resources_policies_repository.get_policies().await?;
        let resource_type_policies = self
            .resource_types_policies_repository
            .get_policies()
            .await?;
        Ok((resource_policies, resource_type_policies))
    }
}

// Methods for resource policies
impl Policies {
    pub async fn store_default_resource_type_policies(&self) -> Result<()> {
        for resource_type in ResourceType::iter() {
            for action in Action::iter() {
                self.store_default_policy_for_resource_type(&resource_type, &action)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn store_policy_for_resource_name(
        &self,
        resource_name: &ResourceName,
        action: &Action,
        expression: &Expr,
    ) -> Result<()> {
        self.resources_policies_repository
            .store_policy(resource_name, action, expression)
            .await
    }

    pub async fn get_policy_for_resource_name(
        &self,
        resource_name: &ResourceName,
        action: &Action,
    ) -> Result<Option<ResourcePolicy>> {
        self.resources_policies_repository
            .get_policy(resource_name, action)
            .await
    }

    pub async fn get_policies_for_resource_name(
        &self,
        resource_name: &ResourceName,
    ) -> Result<Vec<ResourcePolicy>> {
        self.resources_policies_repository
            .get_policies_by_resource_name(resource_name)
            .await
    }

    pub async fn delete_policy_for_resource_name(
        &self,
        resource_name: &ResourceName,
        action: &Action,
    ) -> Result<()> {
        self.resources_policies_repository
            .delete_policy(resource_name, action)
            .await
    }

    pub async fn get_expression_for_resource(
        &self,
        resource: &Resource,
        action: &Action,
    ) -> Result<Option<Expr>> {
        // Try to get a policy for the resource name.
        if let Some(policy) = self
            .get_policy_for_resource_name(&resource.resource_name, action)
            .await?
        {
            return Ok(Some(policy.expression));
        }

        // If there is no policy for the resource name, try to get
        // the policy for the resource type associated to the resource name.
        if let Some(policy) = self
            .get_policy_for_resource_type(&resource.resource_type, action)
            .await?
        {
            return Ok(Some(policy.expression));
        }

        Ok(None)
    }
}

// Methods for resource type policies
impl Policies {
    pub async fn store_policy_for_resource_type(
        &self,
        resource_type: &ResourceType,
        action: &Action,
        expression: &Expr,
    ) -> Result<()> {
        self.resource_types_policies_repository
            .store_policy(resource_type, action, expression)
            .await
    }

    async fn store_default_policy_for_resource_type(
        &self,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<()> {
        let expression = subject_has_credential_policy_expression();
        self.resource_types_policies_repository
            .store_policy(resource_type, action, &expression)
            .await
    }

    pub async fn get_policy_for_resource_type(
        &self,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<Option<ResourceTypePolicy>> {
        self.resource_types_policies_repository
            .get_policy(resource_type, action)
            .await
    }

    pub async fn get_policies_for_resource_type(
        &self,
        resource_type: &ResourceType,
    ) -> Result<Vec<ResourceTypePolicy>> {
        self.resource_types_policies_repository
            .get_policies_by_resource_type(resource_type)
            .await
    }

    pub async fn delete_policy_for_resource_type(
        &self,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<()> {
        self.resource_types_policies_repository
            .delete_policy(resource_type, action)
            .await
    }
}
