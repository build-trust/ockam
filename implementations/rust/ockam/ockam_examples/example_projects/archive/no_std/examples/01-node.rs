#![cfg_attr(all(feature = "alloc", feature = "cortexm"), feature(alloc_error_handler))]
#![cfg_attr(all(not(feature = "std"), feature = "cortexm"), no_std)]
#![cfg_attr(all(not(feature = "std"), feature = "cortexm"), no_main)]


// - bare metal entrypoint ----------------------------------------------------

#[cfg(all(feature = "alloc", feature = "cortexm"))]
mod allocator;

#[cfg(feature = "cortexm")]
use panic_semihosting as _;

#[cfg(feature = "cortexm")]
use cortex_m_semihosting::debug;

#[cfg(feature = "cortexm")]
use ockam::println;

#[cfg(feature = "atsame54")]
use atsame54_xpro as _;

#[cfg(feature = "stm32f4")]
use stm32f4xx_hal as _;

#[cfg(feature = "cortexm")]
#[cortex_m_rt::entry]
fn entry() -> ! {
    #[cfg(feature = "alloc")]
    allocator::init();

    main().unwrap();

    loop { }
}


// - ockam::node entrypoint ---------------------------------------------------

use ockam::{Context, Result};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {

    // Stop the node as soon as it starts.
    println!("Stop the node as soon as it starts.");
    let result = ctx.stop().await;

    // exit qemu
    #[cfg(feature = "cortexm")]
    {
        debug::exit(debug::EXIT_SUCCESS);
    }

    result
}
