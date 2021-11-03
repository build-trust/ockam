#![allow(unused_imports)]
// Test case that passes if ockam is used as `o` (or any other id).
use ockam::{self as o};

#[ockam_node_test_attribute::node]
async fn main(mut c: o::Context) {
    c.stop().await.unwrap();
}
