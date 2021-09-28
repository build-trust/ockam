//! `no_std`, `no_alloc` bump pointer allocator
//!
//! This allocator manages "static" memory, memory that resides in a `static` variable and that will
//! never be deallocated. This allocator never deallocates the memory it allocates.

use core::mem::{self, MaybeUninit};

pub struct Alloc {
    len: usize,
    pos: usize,
    start: *mut u8,
}

impl Alloc {
    pub(crate) fn new(memory: &'static mut [u8]) -> Self {
        Self {
            len: memory.len(),
            pos: 0,
            start: memory.as_mut_ptr(),
        }
    }

    fn alloc<T>(&mut self) -> &'static mut MaybeUninit<T> {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        let new_pos = round_up(self.pos, align);
        if new_pos + size >= self.len {
            // OOM
            crate::executor::abort();
        }
        self.pos = new_pos + size;
        unsafe { &mut *(self.start.add(new_pos) as *mut MaybeUninit<T>) }
    }

    /// Effectively stores `val` in static memory and returns a reference to it
    pub(crate) fn alloc_init<T>(&mut self, val: T) -> &'static mut T {
        let slot = self.alloc::<T>();
        unsafe {
            slot.as_mut_ptr().write(val);
            &mut *slot.as_mut_ptr()
        }
    }
}

/// Rounds up `n` a the nearest multiple `m`
fn round_up(n: usize, m: usize) -> usize {
    let rem = n % m;
    if rem == 0 {
        n
    } else {
        (n + m) - rem
    }
}
