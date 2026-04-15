//! Platform memory layout trait definition

/// Linker script symbols for stack size calculation
unsafe extern "C" {
    /// Total stack size from linker script (calculated as __sstack - __estack)
    static __stack_size: usize;
}

/// Platform memory layout information trait
///
/// This trait provides platform-specific memory layout information using const.
/// Memory addresses (stack and heap) are obtained from linker script symbols.
///
/// # Note
///
/// `MAX_PROGRAMS` is automatically calculated from linker script symbols:
/// `(__stack_size - INTERRUPT_STACK_SIZE) / PROGRAM_STACK_SIZE`
pub trait PlatformMemoryLayout {
    const MAX_PROGRAMS: usize = {
        unsafe { (__stack_size - Self::INTERRUPT_STACK_SIZE) / Self::PROGRAM_STACK_SIZE }
    };

    /// Program stack size in bytes
    const PROGRAM_STACK_SIZE: usize;

    /// Interrupt stack size in bytes
    const INTERRUPT_STACK_SIZE: usize;
}
