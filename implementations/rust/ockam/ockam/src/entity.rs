mod profile;
pub use profile::*;

use hashbrown::HashMap;

#[derive(Clone, Debug)]
pub struct Entity {
    pub profiles: HashMap<ProfileIdentifier, Profile>,
}

impl Entity {
    pub fn new() -> Self {
        Entity {
            profiles: HashMap::new(),
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let _id = Entity::new();
    }
}
