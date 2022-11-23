fn main() {
    let (mut context, mut executor) = ockam::NodeBuilder::new().build();
    executor
        .execute(async move {
            context.stop().await.unwrap();
        })
        .unwrap();
}
