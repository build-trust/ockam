/// Creates a [`CredentialSchema`] containing the arguments.
///
/// ```
/// # use ockam_entity::credential_type;;
/// let schema = credential_type!["TYPE_ID"; "attribute_of_string_type", (Number, "attribute_of_number_type")];
/// ```
///
/// [`CredentialSchema`]: crate::CredentialSchema
#[macro_export]
macro_rules! credential_type {
    ($t:expr; $($x:expr),* $(,)?) => ({
        use $crate::CredentialAttributeSchema;
        use $crate::CredentialAttributeType::{Number, Utf8String, Blob};

        let mut attributes = vec![CredentialAttributeSchema {
            label: $crate::SECRET_ID.into(),
            description: "".to_string(),
            attribute_type: Blob,
            unknown: true,
        }]; // FIXME
        $(attributes.push($x.into());)*

        $crate::CredentialSchema {
            id: $t.into(),
            label: String::new(),
            description: String::new(),
            attributes,
        }
    });
}

/// Creates a [`CredentialAttribute`] vector containing the arguments.
///
/// ```
/// # use ockam_entity::credential_attribute_values;;
/// let values = credential_attribute_values!["ABCD-EFGH", 1];
/// ```
///
/// [`CredentialAttribute`]: crate::CredentialAttribute
#[macro_export]
macro_rules! credential_attribute_values {
    ($($x:expr),* $(,)?) => ({
        use $crate::CredentialAttribute;

        let mut attribute_values: Vec<CredentialAttribute> = vec![];
        $(attribute_values.push($x.into());)*

        attribute_values
    });
}

/// Creates a list of revealed attributes containing the arguments.
///
/// ```
/// # use ockam_entity::reveal_attributes;;
/// let reveal_attributes = reveal_attributes!["ABCD-EFGH", "BDC".to_string()];
/// ```
#[macro_export]
macro_rules! reveal_attributes {
    ($($x:expr),* $(,)?) => ({
        let mut reveal_attributes: Vec<String> = vec![];
        $(reveal_attributes.push($x.into());)*

        reveal_attributes
    });
}
