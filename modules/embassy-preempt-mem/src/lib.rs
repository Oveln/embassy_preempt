//! Memory management for Embassy Preempt RTOS
//!
//! This crate provides memory management functionality including:
//! - Memory arenas
//! - Heap allocation with different strategies
//! - Fixed-size block allocation
//! - Stack-based allocation
//! - Linked list-based heap management

#![no_std]
#![allow(missing_docs)]

pub extern crate alloc;

#[macro_use]
extern crate embassy_preempt_log;

pub mod arena;
pub mod heap;

pub use heap::*;