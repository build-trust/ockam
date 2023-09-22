#![deny(unused_imports)]

use ockam::Context;

#[ockam::node]
async fn main(c: Context) {
    c.stop().await.unwrap();
}
