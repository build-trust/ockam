use ockam::NodeExecutor;

fn main() {
    let mut executor = NodeExecutor::new();
    let context = executor.new_worker_context("test");

    executor
        .execute(async move {
            context.node.stop().await.unwrap();
        })
        .unwrap();
}
