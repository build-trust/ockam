use core::alloc::Layout;
use alloc_cortex_m::CortexMHeap;


// - heap ---------------------------------------------------------------------

const HEAP_SIZE: usize = 1024 * 1536; // in bytes

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();


// - initialization -----------------------------------------------------------

pub fn init() {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE)
    }
}


// - error handler ------------------------------------------------------------

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    cortex_m_semihosting::hprintln!("allocator.rs - alloc error").unwrap();
    cortex_m::asm::bkpt();
    loop {}
}
