//! Platform memory layout trait definition

/// Platform memory layout information trait
///
/// This trait provides platform-specific memory layout information using const.
/// Memory addresses (stack and heap) are obtained from linker script symbols.
pub trait PlatformMemoryLayout {
    /// Maximum number of programs/tasks supported
    const MAX_PROGRAMS: usize;

    /// Program stack size in bytes
    const PROGRAM_STACK_SIZE: usize;

    /// Interrupt stack size in bytes
    const INTERRUPT_STACK_SIZE: usize;
}
