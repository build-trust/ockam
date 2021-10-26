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
use ockam::{
    compat::string::{String, ToString},
    println
};

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

use ockam::{route, Context, Entity, Result, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Vault to safely store secret keys for Alice and Bob.
    let vault = Vault::create(&ctx).await?;

    // Create an Entity to represent Bob.
    let mut bob = Entity::create(&ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob", TrustEveryonePolicy).await?;

    // Create an entity to represent Alice.
    let mut alice = Entity::create(&ctx, &vault).await?;

    // As Alice, connect to Bob's secure channel listener and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel("bob", TrustEveryonePolicy).await?;

    // Send a message, ** THROUGH ** the secure channel,
    // to the "app" worker on the other side.
    //
    // This message will automatically get encrypted when it enters the channel
    // and decrypted just before it exits the channel.
    ctx.send(route![channel, "app"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a message for the "app" worker and print it.
    let message = ctx.receive::<String>().await?;
    println!("App Received: {}", message); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    let result = ctx.stop().await;

    // exit qemu
    #[cfg(feature = "cortexm")]
    {
        debug::exit(debug::EXIT_SUCCESS);
    }

    result
}
