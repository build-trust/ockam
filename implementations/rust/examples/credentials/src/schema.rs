use ockam::{CredentialAttributeSchema, CredentialAttributeType, CredentialSchema, SECRET_ID};

pub fn example_schema() -> CredentialSchema {
    CredentialSchema {
        id: "file:///truck-schema-20210227-1_0_0".to_string(),
        label: "Truck Management".to_string(),
        description: "A Demoable schema".to_string(),
        attributes: vec![
            CredentialAttributeSchema {
                label: SECRET_ID.to_string(),
                description: "A unique identifier for maintenance worker. ".to_string(),
                attribute_type: CredentialAttributeType::Blob,
                unknown: true,
            },
            CredentialAttributeSchema {
                label: "can_access".to_string(),
                description: "Can worker access the truck maintenance codes?".to_string(),
                attribute_type: CredentialAttributeType::Number,
                unknown: false,
            },
        ],
    }
}
