//! Platform trait definitions

pub mod memory_layout;
pub mod platform;
pub mod timer;

// Re-export for convenience
pub use memory_layout::PlatformMemoryLayout;
pub use platform::Platform;
