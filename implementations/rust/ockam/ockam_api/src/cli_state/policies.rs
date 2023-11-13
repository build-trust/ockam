use crate::cli_state::CliState;
use crate::cli_state::Result;
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, Resource};

impl CliState {
    pub async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Expr>> {
        Ok(self.policies_repository().await?.get_policy(r, a).await?)
    }

    pub async fn set_policy(&self, r: &Resource, a: &Action, c: &Expr) -> Result<()> {
        Ok(self
            .policies_repository()
            .await?
            .set_policy(r, a, c)
            .await?)
    }

    pub async fn delete_policy(&self, r: &Resource, a: &Action) -> Result<()> {
        Ok(self
            .policies_repository()
            .await?
            .delete_policy(r, a)
            .await?)
    }

    pub async fn get_policies_by_resource(&self, r: &Resource) -> Result<Vec<(Action, Expr)>> {
        Ok(self
            .policies_repository()
            .await?
            .get_policies_by_resource(r)
            .await?)
    }

    pub async fn make_policy_access_control(
        &self,
        r: &Resource,
        a: &Action,
        env: Env,
    ) -> Result<PolicyAccessControl> {
        let policies = self.policies_repository().await?.clone();
        debug!(
            "set a policy access control for resource '{}' and action '{}'",
            &r, &a
        );

        Ok(PolicyAccessControl::new(
            policies,
            self.identity_attributes_repository().await?,
            r.clone(),
            a.clone(),
            env,
        ))
    }
}
