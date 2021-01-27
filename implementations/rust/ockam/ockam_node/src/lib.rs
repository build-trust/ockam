mod context;
pub use context::*;

mod error;
pub use error::*;

mod executor;
pub use executor::*;

mod node;
pub use node::*;

mod worker;
pub use worker::*;

pub type Address = String;

pub fn node<T>() -> (Context<T>, NodeExecutor<T>) {
    let executor = NodeExecutor::new();
    let context = executor.new_worker_context("node");
    (context, executor)
}
