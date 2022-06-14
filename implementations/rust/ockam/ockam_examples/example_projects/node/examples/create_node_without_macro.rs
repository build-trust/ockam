fn main() {
    let (mut context, mut executor) = ockam::NodeBuilder::without_access_control().build();
    executor
        .execute(async move {
            context.stop().await.unwrap();
        })
        .unwrap();
}
