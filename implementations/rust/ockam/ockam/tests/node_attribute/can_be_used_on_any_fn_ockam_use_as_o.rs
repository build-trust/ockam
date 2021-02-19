// Test case that passes if ockam is used as `o` (or any other id).
use ockam::{self as o};

#[ockam::node]
async fn foo(c: o::Context) {
    c.stop().await.unwrap();
}
