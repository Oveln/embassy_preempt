//! the allocator of the heap

/// Block_Allocator based on Linked_List_Allocator
pub mod fixed_size_block;
/// Linked_List_Allocator for [no_std]
pub mod linked_list;
/// Stack_Allocator for OS_STK
pub mod stack_allocator;
use fixed_size_block::FixedSizeBlockAllocator;
pub use stack_allocator::*;

/// Linker script symbols for heap boundaries
unsafe extern "C" {
    static __sheap: u8;
    static __eheap: u8;
}

/// Get the heap start address from linker script
fn heap_start() -> *mut u8 {
    unsafe { &__sheap as *const u8 as *mut u8 }
}

/// Get the heap size from linker script symbols
fn heap_size() -> usize {
    unsafe { (&__eheap as *const u8 as usize) - (&__sheap as *const u8 as usize) }
}

/// Global allocator
#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

#[allow(unused)]
pub fn Init_Heap() {
    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_start(), heap_size());
    }
    mem_log!(trace, "Init_Heap: completed");
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}
