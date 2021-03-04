use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_router::RouteTransportMessage;
use std::sync::{Arc, Mutex};

pub async fn initiator(ctx: Arc<Mutex<Context>>) {
    let connection = IConnection {};

    let mut ctx = ctx.lock();
    ctx.start(Address::from("connection")).await.unwrap();
}

#[ockam::node]
async fn main(ctx: ockam::Context) {}
