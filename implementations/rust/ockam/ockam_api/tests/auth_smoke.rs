use ockam::identity::{Identifier, IdentityAttributesRepository};
use ockam_api::auth;
use ockam_api::auth::AuthorizationApi;
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_core::{route, Result};
use ockam_node::api::Client;
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;

#[ockam_macros::test]
async fn auth_smoke(ctx: &mut Context) -> Result<()> {
    let s = PreTrustedIdentities::new_from_string(
        r#"{"I124ed0b2e5a2be82e267ead6b3279f683616b66da1b2c3d4e5f6a6b5c4d3e2f1":{"attr":"value"},
            "I224ed0b2e5a2be82e267ead6b3279f683616b66da1b2c3d4e5f6a6b5c4d3e2f1":{"attr":"value2"}
           }"#,
    )?;
    let s: Arc<dyn IdentityAttributesRepository> = Arc::new(s);
    ctx.start_worker("auth", auth::Server::new(s)).await?;

    let client = Client::new(&route!["auth"], None);

    // Retrieve an existing one
    let entry = client
        .get_attributes(
            ctx,
            &Identifier::from_str(
                "I124ed0b2e5a2be82e267ead6b3279f683616b66da1b2c3d4e5f6a6b5c4d3e2f1",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .expect("found");
    assert_eq!(Some("value".into()), entry.get("attr".into()));
    assert_eq!(None, entry.attested_by());
    assert_eq!(None, entry.expires());

    // Try to retrieve non-existing one
    assert_eq!(
        None,
        client
            .get_attributes(
                ctx,
                &Identifier::from_str(
                    "I324ed0b2e5a2be82e267ead6b3279f683616b66da1b2c3d4e5f6a6b5c4d3e2f1"
                )
                .unwrap()
            )
            .await
            .unwrap()
    );

    assert_eq!(2, client.list_identifiers(ctx).await.unwrap().len());

    ctx.stop().await
}
