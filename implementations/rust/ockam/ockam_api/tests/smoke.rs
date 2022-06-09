// This test was broken by commenting out some of the node API types.  Might completely remove this or rewrite it to make sense with the new structure.

// use ockam_api::nodes;
// use ockam_core::Result;
// use ockam_node::Context;

// #[ockam_macros::test]
// async fn smoke(ctx: &mut Context) -> Result<()> {
//     ctx.start_worker("nodes", nodes::Server::default()).await?;

//     let mut client = nodes::Client::new("nodes".into(), ctx).await?;

//     let a = client
//         .create_node(&nodes::types::CreateNode::new("a"))
//         .await?;
//     let i = a.id().to_string();

//     let b = client
//         .create_node(&nodes::types::CreateNode::new("b"))
//         .await?;
//     let j = b.id().to_string();

//     let c = client.get(&i).await?;
//     assert_eq!(i, c.unwrap().id());

//     let c = client.get(&j).await?;
//     assert_eq!(j, c.unwrap().id());

//     let c = client.list().await?;
//     assert_eq!(2, c.len());

//     let c = client.delete(&i).await;
//     assert!(c.is_ok());

//     let c = client.list().await?;
//     assert_eq!(1, c.len());

//     let c = client.get(&j).await?;
//     assert_eq!(j, c.unwrap().id());

//     ctx.stop().await?;
//     Ok(())
// }
