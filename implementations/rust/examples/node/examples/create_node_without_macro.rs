fn main() {
    let (context, mut executor) = ockam::start_node();
    executor
        .execute(async move {
            context.stop().unwrap();
        })
        .unwrap();
}
