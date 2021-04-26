use crate::{Profile, ProfileEventAttributes};
use ockam_core::Address;
use ockam_node::Context;
use ockam_vault_sync_core::VaultSync;

pub struct ProfileBuilder {}

impl ProfileBuilder {
    /// Generate fresh [`Profile`] update key and create new [`Profile`] using it
    pub fn create_with_attributes(
        attributes: Option<ProfileEventAttributes>,
        ctx: &Context,
        vault: &Address,
    ) -> ockam_core::Result<Profile<VaultSync>> {
        let vault = VaultSync::create_with_worker(ctx, vault, "" /* FIXME */)?;

        Profile::create(attributes, vault)
    }

    /// Generate fresh [`Profile`] update key and create new [`Profile`] using it
    pub fn create(ctx: &Context, vault: &Address) -> ockam_core::Result<Profile<VaultSync>> {
        Self::create_with_attributes(None, ctx, vault)
    }
}
