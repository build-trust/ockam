use ockam::abac::{
    self, AbacAttributeStorage, AbacPolicyStorage, Action, Method, Resource, Subject,
};
use ockam::identity::IdentityIdentifier;
use ockam::Result;

/// Populate `AbacPolicyStorage` with some policy data for examples.
pub async fn with_policy_test_data<S: AbacPolicyStorage>(storage: S) -> Result<S> {
    // Set up some conditionals on attributes
    let project_green = abac::eq("project", abac::string("green"));
    let project_blue = abac::eq("project", abac::string("blue"));
    let role_reader = abac::eq("role", abac::string("reader"));
    let role_writer = abac::eq("role", abac::string("writer"));

    // Define some policies
    storage
        .set_policy(
            Resource::from("/project/green/1234"),
            Action::from("read"),
            &project_green.and(&role_reader.or(&role_writer)),
        )
        .await?;

    storage
        .set_policy(
            Resource::from("/project/green/1234"),
            Action::from("write"),
            &project_green.and(&role_writer),
        )
        .await?;

    storage
        .set_policy(
            Resource::from("/project/blue/5678"),
            Action::from("write"),
            &project_blue.and(&role_writer),
        )
        .await?;

    let mut resource = Resource::from("/echoer");
    resource.extend([("space".into(), abac::string("some_customer_space"))]);
    storage
        .set_policy(
            resource,
            Action::from(Method::Post),
            &project_green.and(&role_reader.or(&role_writer)),
        )
        .await?;

    Ok(storage)
}

/// Populate `AbacAttributeStorage` with some attribute data for examples.
pub async fn with_attribute_test_data<S: AbacAttributeStorage>(
    storage: S,
    identifier: IdentityIdentifier,
) -> Result<S> {
    // Set up some subjects with attributes
    storage
        .set_subject_attributes(
            Subject::from(identifier),
            [
                ("role".into(), abac::string("reader")),
                ("project".into(), abac::string("green")),
            ]
            .into(),
        )
        .await?;

    storage
        .set_subject_attributes(
            Subject::from(0x0000_0000_0000_0002),
            [
                ("role".into(), abac::string("writer")),
                ("project".into(), abac::string("green")),
            ]
            .into(),
        )
        .await?;

    storage
        .set_subject_attributes(
            Subject::from(0x0000_0000_0000_0003),
            [
                ("role".into(), abac::string("writer")),
                ("project".into(), abac::string("blue")),
            ]
            .into(),
        )
        .await?;

    Ok(storage)
}
