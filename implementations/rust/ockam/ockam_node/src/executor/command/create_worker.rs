use super::NodeExecutor;

#[derive(Clone, Debug)]
pub struct CreateWorker {
    address: String
};

impl CreateWorker {
    pub fn run(&self, executor: &NodeExecutor) -> bool {
        println!("create worker");

        context = executor.new_worker_context()

        worker = NodeWorker::new(context)


        executor.register_worker()

        false
    }
}
