use crate::cli_state::CliState;
use crate::cli_state::Result;
use ockam_abac::{Action, Env, Policy, PolicyAccessControl, Resource};

impl CliState {
    pub async fn get_policy(&self, resource: &Resource, action: &Action) -> Result<Option<Policy>> {
        Ok(self
            .policies_repository()
            .await?
            .get_policy(resource, action)
            .await?)
    }

    pub async fn set_policy(
        &self,
        resource: &Resource,
        action: &Action,
        policy: &Policy,
    ) -> Result<()> {
        Ok(self
            .policies_repository()
            .await?
            .set_policy(resource, action, policy)
            .await?)
    }

    pub async fn delete_policy(&self, resource: &Resource, action: &Action) -> Result<()> {
        Ok(self
            .policies_repository()
            .await?
            .delete_policy(resource, action)
            .await?)
    }

    pub async fn get_policies_by_resource(
        &self,
        resource: &Resource,
    ) -> Result<Vec<(Action, Policy)>> {
        Ok(self
            .policies_repository()
            .await?
            .get_policies_by_resource(resource)
            .await?)
    }

    pub async fn make_policy_access_control(
        &self,
        resource: &Resource,
        action: &Action,
        env: Env,
    ) -> Result<PolicyAccessControl> {
        let policies = self.policies_repository().await?.clone();
        debug!(
            "set a policy access control for resource '{}' and action '{}'",
            &resource, &action
        );

        Ok(PolicyAccessControl::new(
            policies,
            self.identity_attributes_repository().await?,
            resource.clone(),
            action.clone(),
            env,
        ))
    }
}
