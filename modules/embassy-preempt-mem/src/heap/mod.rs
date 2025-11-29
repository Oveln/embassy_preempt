//! the allocator of the heap

/// Block_Allocator based on Linked_List_Allocator
pub mod fixed_size_block;
/// Linked_List_Allocator for [no_std]
pub mod linked_list;
/// Stack_Allocator for OS_STK
pub mod stack_allocator;

use embassy_preempt_platform::chip::PlatformImpl;
use embassy_preempt_platform::traits::memory_layout::PlatformMemoryLayout;
use fixed_size_block::FixedSizeBlockAllocator;
pub use stack_allocator::*;

/// Global allocator
#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());
#[allow(unused)]
pub fn Init_Heap() {
    mem_log!(
        trace,
        "Init_Heap: start={:x}, size={}",
        PlatformImpl::get_heap_start(),
        PlatformImpl::get_heap_size()
    );
    unsafe {
        ALLOCATOR
            .lock()
            .init(PlatformImpl::get_heap_start(), PlatformImpl::get_heap_size());
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
