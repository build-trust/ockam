use crate::authenticator::{Member, MembersStorage};
use ockam::identity::Identifier;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, Result};

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembersStorage {
    map: Arc<RwLock<HashMap<Identifier, Member>>>,
}

impl InMemoryMembersStorage {
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl MembersStorage for InMemoryMembersStorage {
    async fn get_member(&self, identifier: &Identifier) -> Result<Option<Member>> {
        Ok(self.map.read().unwrap().get(identifier).cloned())
    }

    async fn get_members(&self) -> Result<Vec<Member>> {
        Ok(self.map.read().unwrap().values().cloned().collect())
    }

    async fn delete_member(&self, identifier: &Identifier) -> Result<Option<Member>> {
        let mut map = self.map.write().unwrap();
        if let Some(member) = map.remove(identifier) {
            if !member.is_permanent() {
                Ok(Some(member))
            } else {
                map.insert(member.identifier().clone(), member);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn add_member(&self, member: Member) -> Result<()> {
        self.map
            .write()
            .unwrap()
            .insert(member.identifier().clone(), member);

        Ok(())
    }
}
