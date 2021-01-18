#[derive(Clone, Debug)]
pub struct ProfileIdentifier(Vec<u8>);

impl ProfileIdentifier {
    pub fn new() -> Self {
        ProfileIdentifier(vec![])
    }
}

impl Default for ProfileIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let _identifier = ProfileIdentifier::new();
    }
}
