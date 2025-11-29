//! Platform memory layout trait definition

/// Platform memory layout information trait
///
/// This trait provides platform-specific memory layout information including
/// stack and heap memory regions, sizes, and configuration.
pub trait PlatformMemoryLayout {
    /// Get the stack memory start address
    ///
    /// Returns the starting address of the stack memory region.
    /// This address is platform-specific and depends on the memory map.
    fn get_stack_start() -> usize;

    /// Get the maximum number of programs/tasks supported
    ///
    /// Returns the maximum number of concurrent programs/tasks that can be supported.
    /// Stack size is calculated as: INTERRUPT_STACK_SIZE + PROGRAM_STACK_SIZE * MAX_PROGRAMS
    fn get_max_programs() -> usize;

    /// Get the heap memory size
    ///
    /// Returns the total size of the heap memory region in bytes.
    fn get_heap_size() -> usize;

    /// Get the program stack size
    ///
    /// Returns the size allocated for each task's program stack in bytes.
    fn get_program_stack_size() -> usize;

    /// Get the interrupt stack size
    ///
    /// Returns the size allocated for the interrupt stack in bytes.
    fn get_interrupt_stack_size() -> usize;

    /// Get the task stack size
    ///
    /// Returns the size allocated for task stacks in bytes.
    /// Currently set to the same as program stack size.
    fn get_task_stack_size() -> usize {
        Self::get_program_stack_size()
    }

    /// Calculate total stack size based on configuration
    ///
    /// Calculates the total stack size as: INTERRUPT_STACK_SIZE + PROGRAM_STACK_SIZE * MAX_PROGRAMS
    fn calculate_stack_size() -> usize {
        Self::get_interrupt_stack_size() + (Self::get_program_stack_size() * Self::get_max_programs())
    }

    /// Get the heap memory start address
    ///
    /// Returns the starting address of the heap memory region.
    /// This is typically located after the stack region in memory.
    fn get_heap_start() -> *mut u8 {
        (Self::get_stack_start() + Self::calculate_stack_size()) as *mut u8
    }
}
