/*
********************************************************************************************************************************************
*                                                               type define
********************************************************************************************************************************************
*/

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;

use embassy_preempt_platform::chip::PlatformImpl;
use embassy_preempt_traits::PlatformMemoryLayout;
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_structs::cell::UPSafeCell;
use spin::Once;

use super::Locked;
use super::fixed_size_block::FixedSizeBlockAllocator;
static STACK_ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());
static PROGRAM_STACK: Once<UPSafeCell<OS_STK_REF>> = Once::new();
static INTERRUPT_STACK: Once<UPSafeCell<OS_STK_REF>> = Once::new();

/// Linker script symbols for stack boundaries
unsafe extern "C" {
    static __sstack: u8;
    static __estack: u8;
}

/// Get the stack start address from linker script
fn stack_start() -> *mut u8 {
    unsafe { &__estack as *const u8 as *mut u8 }
}

/// Get the stack size from linker script symbols
fn stack_size() -> usize {
    unsafe { (&__sstack as *const u8 as usize) - (&__estack as *const u8 as usize) }
}

/// Get access to the program stack
pub fn get_program_stack() -> &'static UPSafeCell<OS_STK_REF> {
    PROGRAM_STACK.get().expect("PROGRAM_STACK not initialized")
}

/// Get access to the interrupt stack
pub fn get_interrupt_stack() -> &'static UPSafeCell<OS_STK_REF> {
    INTERRUPT_STACK.get().expect("INTERRUPT_STACK not initialized")
}

/*
********************************************************************************************************************************************
*                                                           interface
********************************************************************************************************************************************
*/
/// init the stack allocator and set up the program stack and the interrupt stack
pub fn OS_InitStackAllocator() {
    mem_log!(trace, "Init Stack Allocator at 0x{:x}, size: 0x{:x}", stack_start() as usize,stack_size());
    unsafe {
        STACK_ALLOCATOR.lock().init(
            stack_start(),
            stack_size(),
        );
    }
    // allocate interrupt Stack and set the interrupt stack pointe
    let layout = Layout::from_size_align(PlatformImpl::INTERRUPT_STACK_SIZE, 4).unwrap();
    let stk = alloc_stack(layout);
    INTERRUPT_STACK.call_once(|| unsafe { UPSafeCell::new(stk) });

    // allocate program stack
    let layout = Layout::from_size_align(PlatformImpl::PROGRAM_STACK_SIZE, 4).unwrap();
    let stk = alloc_stack(layout);
    let stk_ptr = stk.STK_REF.as_ptr() as *mut u8;
    PROGRAM_STACK.call_once(|| unsafe { UPSafeCell::new(stk) });
    // then we change the sp to the top of the program stack
    // this depending on the arch so we need extern and implement in the port
    embassy_preempt_platform::set_program_stack_pointer(stk_ptr);
}
/// alloc a new stack
pub fn alloc_stack(layout: Layout) -> OS_STK_REF {
    mem_log!(trace, "alloc_stack");
    let heap_ptr: *mut u8;
    unsafe {
        heap_ptr = STACK_ALLOCATOR.alloc(layout);
    }
    //
    // mem_log!(trace, "alloc a stack at {:p}", heap_ptr);
    stk_from_ptr(heap_ptr, layout)
}
/// dealloc a stack
pub fn dealloc_stack(stk: &mut OS_STK_REF) {
    mem_log!(trace, "dealloc_stack");
    if stk.STK_REF == NonNull::dangling() || stk.HEAP_REF == NonNull::dangling() {
        return;
    }
    let stk_ptr = stk.HEAP_REF.as_ptr();
    stk.STK_REF = NonNull::dangling();
    stk.HEAP_REF = NonNull::dangling();
    unsafe {
        STACK_ALLOCATOR.dealloc(stk_ptr, stk.layout);
    }
}

/// the ref of the stk
pub struct OS_STK_REF {
    /// the ref of the stk(top or bottom),because the read of this
    /// field is in the asm code, so we use NonNull to ensure the safety
    /// and use #[allow(dead_code)]
    #[allow(dead_code)]
    pub STK_REF: NonNull<usize>,
    /// the ref of this dynamic stk's src heap
    pub HEAP_REF: NonNull<u8>,
    /// the layout(size) of the stk
    pub layout: Layout,
}
unsafe impl Send for OS_STK_REF {}

impl Clone for OS_STK_REF {
    fn clone(&self) -> Self {
        OS_STK_REF {
            STK_REF: self.STK_REF,
            HEAP_REF: self.HEAP_REF,
            layout: self.layout,
        }
    }
}

/// when the OS_STK_REF is default, we will not alloc a stack
impl Default for OS_STK_REF {
    fn default() -> Self {
        OS_STK_REF {
            STK_REF: NonNull::dangling(),
            HEAP_REF: NonNull::dangling(),
            layout: Layout::from_size_align(0, 1).unwrap(),
        }
    }
}
/// we impl drop for OS_STK_REF to dealloc the stack(try to be RAII)
impl Drop for OS_STK_REF {
    fn drop(&mut self) {
        if self.STK_REF == NonNull::dangling() || self.HEAP_REF == NonNull::dangling() {
            return;
        }
        let stk_ptr = self.HEAP_REF.as_ptr();
        self.STK_REF = NonNull::dangling();
        self.HEAP_REF = NonNull::dangling();
        unsafe {
            STACK_ALLOCATOR.dealloc(stk_ptr, self.layout);
        }
    }
}

impl OS_STK_REF {
    pub fn as_ptr(&self) -> *mut u8 {
        self.HEAP_REF.as_ptr()
    }
}

pub fn stk_from_ptr(heap_ptr: *mut u8, layout: Layout) -> OS_STK_REF {
    OS_STK_REF {
        STK_REF: NonNull::new(unsafe { heap_ptr.offset(layout.size() as isize) as *mut usize }).unwrap(),
        HEAP_REF: NonNull::new(heap_ptr).unwrap(),
        layout,
    }
}
