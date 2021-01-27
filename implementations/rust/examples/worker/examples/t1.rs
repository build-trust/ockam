#[ockam::node]
async fn main(context: ockam::Context) {
    let node = context.node();

    node.create_worker(async move {
        println!("test");
    }).await;

    node.stop().await.unwrap();
}
