fn main() {
    let (context, mut executor) = ockam::node();
    executor
        .execute(async move {
            context.node().stop().unwrap();
        })
        .unwrap();
}
