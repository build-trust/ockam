mod context;
pub use context::*;

mod error;
pub use error::*;

mod executor;
pub use executor::*;

mod node;
pub use node::*;

pub fn node() -> (Context, NodeExecutor) {
    let executor = NodeExecutor::new();
    let context = executor.new_worker_context();
    (context, executor)
}
