mod profile;
pub use profile::*;

use hashbrown::HashMap;

#[derive(Clone, Debug)]
pub struct Entity {
    profiles: HashMap<ProfileIdentifier, Profile>,
}

impl Entity {
    pub fn profiles(&self) -> &HashMap<ProfileIdentifier, Profile> {
        &self.profiles
    }
}

impl Entity {
    pub fn new(profiles: HashMap<ProfileIdentifier, Profile>) -> Self {
        Entity { profiles }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let _id = Entity::default();
    }
}
