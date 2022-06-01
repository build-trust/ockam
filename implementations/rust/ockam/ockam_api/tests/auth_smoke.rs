use ockam_api::auth;
use ockam_api::auth::store;
use ockam_api::auth::types::Attributes;
use ockam_core::Result;
use ockam_node::Context;

#[ockam_macros::test]
async fn auth_smoke(ctx: &mut Context) -> Result<()> {
    #[cfg(not(feature = "lmdb"))]
    {
        let s = store::mem::Store::new();
        ctx.start_worker("auth", auth::Server::new(s)).await?;
    }

    #[cfg(feature = "lmdb")]
    let tempfile = tempfile::NamedTempFile::new().map_err(|e| {
        use ockam_core::errcode::{Kind, Origin};
        ockam_core::Error::new(Origin::Other, Kind::Io, e)
    })?;

    #[cfg(feature = "lmdb")]
    {
        let s = store::lmdb::Store::new(tempfile.path()).await?;
        ctx.start_worker("auth", auth::Server::new(s)).await?;
    }

    let mut client = auth::Client::new("auth".into(), ctx).await?;

    let mut attrs = Attributes::new();
    attrs
        .put("a", &b"hello"[..])
        .put("b", &b"world"[..])
        .put("c", &b"42"[..]);

    client.set("foo", &attrs).await?;

    assert_eq!(Some(&b"hello"[..]), client.get("foo", "a").await?);
    assert_eq!(Some(&b"world"[..]), client.get("foo", "b").await?);

    client.del("foo", "a").await?;
    assert_eq!(None, client.get("foo", "a").await?);

    ctx.stop().await?;
    Ok(())
}
