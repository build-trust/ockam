use ockam_api::auth;
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_core::{AllowAll, Result};
use ockam_identity::authenticated_storage::IdentityAttributeStorageReader;
use ockam_node::Context;
use std::sync::Arc;

#[ockam_macros::test]
async fn auth_smoke(ctx: &mut Context) -> Result<()> {
    let s = PreTrustedIdentities::new_from_string(
        r#"{"P624ed0b2e5a2be82e267ead6b3279f683616b66de9537a23e45343c95cbb357a":{"attr":"value"},
            "P624ed0b2e5a2be82e267ead6b3279f683616b66de9537a23e45343c95cbb357b":{"attr":"value2"}
           }"#,
    )?;
    let s: Arc<dyn IdentityAttributeStorageReader> = Arc::new(s);
    ctx.start_worker("auth", auth::Server::new(s), AllowAll, AllowAll)
        .await?;

    let mut client = auth::Client::new("auth".into(), ctx).await?;

    // Retrieve an existing one
    let entry = client
        .get("P624ed0b2e5a2be82e267ead6b3279f683616b66de9537a23e45343c95cbb357a")
        .await?
        .expect("found");
    assert_eq!(Some(&b"value"[..].to_vec()), entry.attrs().get("attr"));
    assert_eq!(None, entry.attested_by());
    assert_eq!(None, entry.expires());

    // Try to retrieve non-existing one
    assert_eq!(
        None,
        client
            .get("P111ed0b2e5a2be82e267ead6b3279f683616b66de9537a23e45343c95cbb357b")
            .await?
    );

    assert_eq!(2, client.list().await?.len());

    ctx.stop().await
}
