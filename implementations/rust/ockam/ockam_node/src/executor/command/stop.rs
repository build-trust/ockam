use super::NodeExecutor;

#[derive(Clone, Debug)]
pub struct Stop;

impl Stop {
    pub fn run(&self, _executor: &NodeExecutor) -> bool {
        println!("stopping");
        true
    }
}
