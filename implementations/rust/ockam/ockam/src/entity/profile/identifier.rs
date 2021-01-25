#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProfileIdentifier([u8; 32]);

impl AsRef<[u8]> for ProfileIdentifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ProfileIdentifier {
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { 0: hash }
    }

    pub fn to_string_representation(&self) -> String {
        format!("P_ID.{}", hex::encode(&self.0))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let _identifier = ProfileIdentifier::from_hash([0u8; 32]);
    }
}
