//! Platform memory layout trait definition


/// Platform memory layout information trait
///
/// This trait provides platform-specific memory layout information using const.
pub trait PlatformMemoryLayout {

    /// Program stack size in bytes
    const PROGRAM_STACK_SIZE: usize;

    /// Interrupt stack size in bytes
    const INTERRUPT_STACK_SIZE: usize;
}
