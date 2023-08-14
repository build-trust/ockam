use ockam::identity::IdentityAttributesReader;
use ockam_api::auth;
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_core::Result;
use ockam_node::Context;
use std::sync::Arc;

#[ockam_macros::test]
async fn auth_smoke(ctx: &mut Context) -> Result<()> {
    let s = PreTrustedIdentities::new_from_string(
        r#"{"I124ed0b2e5a2be82e267ead6b3279f683616b66d":{"attr":"value"},
            "I224ed0b2e5a2be82e267ead6b3279f683616b66d":{"attr":"value2"}
           }"#,
    )?;
    let s: Arc<dyn IdentityAttributesReader> = Arc::new(s);
    ctx.start_worker("auth", auth::Server::new(s)).await?;

    let mut client = auth::Client::new("auth".into(), ctx).await?;

    // Retrieve an existing one
    let entry = client
        .get("I124ed0b2e5a2be82e267ead6b3279f683616b66d")
        .await?
        .expect("found");
    assert_eq!(
        Some(&b"value"[..].to_vec()),
        entry.attrs().get("attr".as_bytes())
    );
    assert_eq!(None, entry.attested_by());
    assert_eq!(None, entry.expires());

    // Try to retrieve non-existing one
    assert_eq!(
        None,
        client
            .get("I324ed0b2e5a2be82e267ead6b3279f683616b66d")
            .await?
    );

    assert_eq!(2, client.list().await?.len());

    ctx.stop().await
}
