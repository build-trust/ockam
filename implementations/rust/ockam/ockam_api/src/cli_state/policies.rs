use crate::cli_state::CliState;
use crate::cli_state::Result;
use ockam::identity::Identifier;
use ockam_abac::{Action, Env, Policy, PolicyAccessControl, Resource};

impl CliState {
    pub async fn get_policy(&self, resource: &Resource, action: &Action) -> Result<Option<Policy>> {
        Ok(self
            .policies_repository()
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
            .set_policy(resource, action, policy)
            .await?)
    }

    pub async fn delete_policy(&self, resource: &Resource, action: &Action) -> Result<()> {
        Ok(self
            .policies_repository()
            .delete_policy(resource, action)
            .await?)
    }

    pub async fn get_policies_by_resource(
        &self,
        resource: &Resource,
    ) -> Result<Vec<(Action, Policy)>> {
        Ok(self
            .policies_repository()
            .get_policies_by_resource(resource)
            .await?)
    }

    pub async fn make_policy_access_control(
        &self,
        resource: &Resource,
        action: &Action,
        env: Env,
        authority: Identifier,
    ) -> Result<PolicyAccessControl> {
        let policies = self.policies_repository();
        debug!(
            "set a policy access control for resource '{}' and action '{}'",
            &resource, &action
        );

        Ok(PolicyAccessControl::new(
            policies,
            self.identity_attributes_repository(),
            authority,
            resource.clone(),
            action.clone(),
            env,
        ))
    }
}
