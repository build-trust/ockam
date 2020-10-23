// / src/main.rs

// std and main are not available for bare metal software
#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

extern crate alloc;
extern crate alloc_cortex_m;



use stm32f4xx_hal as hal;
use crate::hal::{prelude::*, stm32};

/*
 * TODO: fix
 * there is a global allocator that the ockam_vault config 
 * requires or it refuses to compile, so this one is disabled.is
 * The allocator still is initialized here, but there needs to be
 * caution if changing allocators on one side vs the other.
 * in general one would want to allow the library user to select the
 * global_allocator implementation without imposing the requirement
 * for a specific one from the library
 */
// use alloc_cortex_m::CortexMHeap;
// #[global_allocator]
// static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

/* these symbols should come from the link script */
extern "C" {
    static mut _heap_start: u32;
    static mut _heap_end:   u32;
}

fn init_heap()
{
    use ockam_vault::ALLOCATOR;

    let start = unsafe { &mut _heap_start as *mut u32 as usize };
    let end   = unsafe { &mut _heap_end   as *mut u32 as usize };
    unsafe { ALLOCATOR.init(start, end - start) }

    // test sequence
    // use alloc::vec::Vec;
    // let mut xs = Vec::new();
    // xs.push(1);
}

use cortex_m_rt::entry; // The runtime

#[allow(unused_imports)]
use panic_halt as _;
// use panic_halt; // When a panic occurs, stop the microcontroller


extern crate ockam_vault;


fn run_vault()
{
    use crate::ockam_vault::{
        Vault,
        types::SecretKeyAttributes,
        types::SecretKeyType,
        types::SecretPersistenceType,
        types::SecretPurposeType,
        // types::SecretKeyContext,
        software::DefaultVault,
    };

    let mut v = DefaultVault::default();

    let ska = SecretKeyAttributes {
        xtype:       SecretKeyType::Buffer(1024),
        persistence: SecretPersistenceType::Ephemeral,
        purpose:     SecretPurposeType::KeyAgreement,
    };
    let skc = v.secret_generate(ska).expect("Couldn't generate secret");
    let _export = v.secret_export(skc).expect("Couldn't export secret");
}


// This marks the entrypoint of our application. The cortex_m_rt creates some
// startup code before this, but we don't need to worry about this
#[entry]
fn main() -> ! {
    init_heap();

   if let (Some(dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the LED. On the Nucleo-446RE it's connected to pin PA5.
        let gpioa = dp.GPIOA.split();
        let mut led = gpioa.pa5.into_push_pull_output();

        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Create a delay abstraction based on SysTick
        let mut delay = hal::delay::Delay::new(cp.SYST, clocks);

        let mut once = true;
        loop {
            // signs of life, if leds continue blinking, run_vault executed
            // without a halting panic.
            
            // On for 1s, off for 1s.
            led.set_high().unwrap();
            delay.delay_ms(1000_u32);
            led.set_low().unwrap();
            delay.delay_ms(1000_u32);

            if once {
                once = false;
                run_vault();
            }
        }
    }

    loop {}
}
