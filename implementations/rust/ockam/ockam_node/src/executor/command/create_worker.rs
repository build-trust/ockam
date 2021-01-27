use super::NodeExecutor;

#[derive(Clone, Debug)]
pub struct CreateWorker;

impl CreateWorker {
    pub fn run(&self, _executor: &NodeExecutor) -> bool {
        println!("create worker");
        false
    }
}
