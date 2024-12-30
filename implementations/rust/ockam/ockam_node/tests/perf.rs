use ockam_core::Result;
use ockam_core::{Address, AsyncTryClone};
use ockam_node::workers::Echoer;
use ockam_node::{Context, NodeBuilder};
use tokio::time::Instant;

#[allow(non_snake_case)]
#[test]
#[ignore]
fn start_and_stop_node__many_times__measure_time() {
    let n_iterations = 10;
    let n_workers = 100;

    let t1 = Instant::now();

    for _ in 0..n_iterations {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                for _ in 0..n_workers {
                    ctx.start_worker(Address::random_local(), Echoer).await?;
                }

                ctx.stop().await?;

                Ok::<(), ockam_core::Error>(())
            })
            .unwrap()
            .unwrap();
    }

    let t2 = Instant::now();

    println!("Time: {:?}", t2 - t1);
}

#[allow(non_snake_case)]
#[ockam_macros::test]
#[ignore]
async fn start_and_stop_workers__many_times__measure_time(ctx: &mut Context) -> Result<()> {
    let n_workers = 5000;

    let mut addresses = Vec::with_capacity(n_workers);

    for _ in 0..n_workers {
        addresses.push(Address::random_local());
    }

    let t1 = Instant::now();

    for address in &addresses {
        ctx.start_worker(address.clone(), Echoer).await?;
    }

    for address in &addresses {
        ctx.stop_address_impl(address)?;
    }

    let t2 = Instant::now();

    println!("Time: {:?}", t2 - t1);

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
#[ignore]
async fn send_messages__many_times__measure_time(ctx: &mut Context) -> Result<()> {
    let n_messages = 500;
    let n_workers = 500;

    let mut addresses = Vec::with_capacity(n_workers);

    for _ in 0..n_workers {
        addresses.push(Address::random_local());
    }

    let msg = "Hello, Ockam".to_string();

    for address in &addresses {
        ctx.start_worker(address.clone(), Echoer).await?;
    }

    let t1 = Instant::now();

    for _ in 0..n_messages {
        for address in &addresses {
            let _msg: String = ctx.send_and_receive(address.clone(), msg.clone()).await?;
        }
    }

    let t2 = Instant::now();

    println!("Time: {:?}", t2 - t1);

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
#[ignore]
async fn create_workers_and_send_messages__many_times__measure_time(
    ctx: &mut Context,
) -> Result<()> {
    let n_messages = 100;
    let n_workers = 500;
    let n_workers_parallel = 50;

    let mut addresses = Vec::with_capacity(n_workers);

    for _ in 0..n_workers {
        addresses.push(Address::random_local());
    }

    for address in &addresses {
        ctx.start_worker(address.clone(), Echoer).await?;
    }

    let msg = "Hello, Ockam".to_string();

    let child_ctx = ctx.async_try_clone().await?;
    ctx.runtime().spawn(async move {
        loop {
            let mut addresses = Vec::with_capacity(n_workers_parallel);

            for _ in 0..n_workers_parallel {
                addresses.push(Address::random_local());
            }

            for address in &addresses {
                child_ctx
                    .start_worker(address.clone(), Echoer)
                    .await
                    .unwrap();
            }

            for address in &addresses {
                child_ctx.stop_address_impl(address).unwrap();
            }
        }
    });

    let t1 = Instant::now();

    for _ in 0..n_messages {
        for address in &addresses {
            let _msg: String = ctx.send_and_receive(address.clone(), msg.clone()).await?;
        }
    }

    let t2 = Instant::now();

    println!("Time: {:?}", t2 - t1);

    Ok(())
}
