//! the allocator of TCB only will be used by executor

use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::mem::MaybeUninit;
use core::ptr::null_mut;

use embassy_preempt::cfg::OS_ARENA_SIZE;

// Import logging macros when logging is enabled
use critical_section::{CriticalSection, Mutex};

/// Every TCB(here, we store TaskStorage) will be stored here.
pub static ARENA: Arena<{ OS_ARENA_SIZE }> = Arena::new();

/// The stack allocator defination of uC/OS-II.
pub struct Arena<const N: usize> {
    buf: UnsafeCell<MaybeUninit<[u8; N]>>,
    ptr: Mutex<Cell<*mut u8>>,
}

unsafe impl<const N: usize> Sync for Arena<N> {}
unsafe impl<const N: usize> Send for Arena<N> {}

impl<const N: usize> Arena<N> {
    const fn new() -> Self {
        Self {
            buf: UnsafeCell::new(MaybeUninit::uninit()),
            ptr: Mutex::new(Cell::new(null_mut())),
        }
    }
    /// alloc the stack memory for the task(TCB) list.
    pub fn alloc<T>(&'static self, cs: CriticalSection) -> &'static mut MaybeUninit<T> {
        mem_log!(trace, "alloc of Arena");
        let layout = Layout::new::<T>();

        let start = self.buf.get().cast::<u8>();
        let end = unsafe { start.add(N) };

        let mut ptr = self.ptr.borrow(cs).get();
        if ptr.is_null() {
            ptr = self.buf.get().cast::<u8>();
        }

        let bytes_left = (end as usize) - (ptr as usize);
        let align_offset = (ptr as usize).next_multiple_of(layout.align()) - (ptr as usize);

        if align_offset + layout.size() > bytes_left {
            panic!("embassy-executor: task arena is full. You must increase the arena size, see the documentation for details: https://docs.embassy.dev/embassy-executor/");
        }
        let res = unsafe { ptr.add(align_offset) };
        let ptr = unsafe { ptr.add(align_offset + layout.size()) };

        self.ptr.borrow(cs).set(ptr);
        unsafe { &mut *(res as *mut MaybeUninit<T>) }
    }
    /// deallocate the most recently allocated memory block
    pub fn dealloc<T>(&'static self, cs: CriticalSection, ptr: *mut T) {
        
        task_log!(trace, "dealloc of Arena");
        let layout = Layout::new::<T>();

        let start = self.buf.get().cast::<u8>();
        let end = unsafe { start.add(N) };

        let current_ptr = self.ptr.borrow(cs).get();
        if ptr.is_null() || (ptr as usize >= end as usize) {
            panic!("Invalid dealloc: Pointer is out of bounds or null.");
        }
        // convert ptr to u8* and check if it's the last allocated block
        let ptr_u8 = ptr as *mut u8;
        let prev_ptr = unsafe { ptr_u8.sub(layout.size()) };

        if prev_ptr == current_ptr {
            // if it's the last allocated block, roll back the allocation pointer
            self.ptr.borrow(cs).set(ptr as *mut u8);
        } else {
            panic!("Non-LIFO deallocation is not supported in this arena allocator.");
        }
    }
}
