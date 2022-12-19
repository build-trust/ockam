use ockam::authenticated_storage::{AuthenticatedStorage, InMemoryStorage};
use ockam_api::auth;
use ockam_core::{AllowAll, Result};
use ockam_node::Context;

#[ockam_macros::test]
async fn auth_smoke(ctx: &mut Context) -> Result<()> {
    let s = InMemoryStorage::new();
    ctx.start_worker("auth", auth::Server::new(s.clone()), AllowAll, AllowAll)
        .await?;

    let mut client = auth::Client::new("auth".into(), ctx).await?;

    s.set("foo", "a".to_string(), b"hello".to_vec()).await?;
    s.set("foo", "b".to_string(), b"world".to_vec()).await?;

    assert_eq!(Some(&b"hello"[..]), client.get("foo", "a").await?);
    assert_eq!(Some(&b"world"[..]), client.get("foo", "b").await?);

    client.del("foo", "a").await?;
    assert_eq!(None, client.get("foo", "a").await?);

    ctx.stop().await
}
