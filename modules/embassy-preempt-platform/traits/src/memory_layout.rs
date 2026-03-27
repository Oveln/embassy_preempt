//! Platform memory layout trait definition

/// Platform memory layout information trait
///
/// This trait provides platform-specific memory layout information using const.
pub trait PlatformMemoryLayout {
    /// Stack memory start address
    const STACK_START: usize;

    /// Maximum number of programs/tasks supported
    const MAX_PROGRAMS: usize;

    /// Heap memory size in bytes
    const HEAP_SIZE: usize;

    /// Program stack size in bytes
    const PROGRAM_STACK_SIZE: usize;

    /// Interrupt stack size in bytes
    const INTERRUPT_STACK_SIZE: usize;

    // ===== Computed Values =====

    /// Get the task stack size (same as program stack size)
    fn task_stack_size() -> usize {
        Self::PROGRAM_STACK_SIZE
    }

    /// Calculate total stack size: INTERRUPT_STACK_SIZE + PROGRAM_STACK_SIZE * MAX_PROGRAMS
    fn calculate_stack_size() -> usize {
        Self::INTERRUPT_STACK_SIZE + (Self::PROGRAM_STACK_SIZE * Self::MAX_PROGRAMS)
    }

    /// Get the heap memory start address (after stack region)
    fn heap_start() -> *mut u8 {
        (Self::STACK_START + Self::calculate_stack_size()) as *mut u8
    }
}
