#![deny(unused_imports)]

use ockam::Context;

#[ockam::node]
async fn main(mut c: Context) {
    c.stop().await.unwrap();
}
