#[cfg(feature = "std")]
#[macro_export]
macro_rules! wait_ms {
    ($millis:expr) => {
        std::thread::sleep(core::time::Duration::from_millis($millis));
    };
}

// TODO convert baremetal implementations to real ms delays

#[cfg(all(not(feature = "std"), target_arch = "arm"))]
#[macro_export]
macro_rules! wait_ms {
    ($millis:expr) => {
        let cycles: u32 = 10_000 * $millis;
        for _ in 0..cycles {
            cortex_m::asm::nop();
        }
    };
}

#[cfg(all(not(feature = "std"), target_arch = "mips"))]
#[macro_export]
macro_rules! wait_ms {
    ($millis:expr) => {
        let cycles = 50_000 * $millis;
        let n = cycles / 2;
        for _ in 0..n {
            unsafe {
                asm!("NOP");
            }
        }
    };
}
