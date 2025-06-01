#![cfg_attr(
    all(feature = "alloc", feature = "cortexm"),
    feature(alloc_error_handler)
)]
#![cfg_attr(all(not(feature = "std"), feature = "cortexm"), no_std)]
#![cfg_attr(all(not(feature = "std"), feature = "cortexm"), no_main)]

use tracing::{error, info};

// - bare metal entrypoint ----------------------------------------------------

#[cfg(all(feature = "alloc", feature = "cortexm"))]
mod allocator;

#[cfg(feature = "cortexm")]
use panic_semihosting as _;

#[cfg(feature = "atsame54")]
use atsame54_xpro as _;

#[cfg(feature = "stm32f4")]
use stm32f4xx_hal as _;

#[cfg(feature = "cortexm")]
#[cortex_m_rt::entry]
fn entry() -> ! {
    // initialize allocator
    #[cfg(feature = "alloc")]
    allocator::init();

    // register tracing subscriber
    #[cfg(feature = "cortexm")]
    {
        use hello_ockam_no_std::tracing_subscriber;
        tracing_subscriber::register();
    }

    // execute main program entry point
    match main() {
        Ok(_) => (),
        Err(e) => {
            error!("Error executing main program entry point: {:?}", e);
        }
    }

    // exit qemu
    #[cfg(feature = "cortexm")]
    {
        use cortex_m_semihosting::debug;
        debug::exit(debug::EXIT_SUCCESS);
    }

    loop {}
}

// - ockam::node entrypoint ---------------------------------------------------

use ockam::{node, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let mut node = node(ctx).await?;

    // Stop the node as soon as it starts.
    info!("Stop the node as soon as it starts.");
    node.shutdown().await;
}
