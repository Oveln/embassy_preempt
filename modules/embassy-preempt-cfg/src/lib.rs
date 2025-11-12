#![no_std]
#![no_main]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

/// the mod which define the data structure of uC/OS-II kernel
pub mod ucosii; 
/// timer timebase tick
mod tick;

use ucosii::OS_PRIO;
use embassy_preempt_structs::cell::UPSafeCell;
// TODO: Make all the config to be feature!!!

/// the const val define the lowest prio
pub const OS_LOWEST_PRIO: OS_PRIO = 63;
/// Size of task variables array (#of INT32U entries)
pub const OS_TASK_REG_TBL_SIZE: usize = 1;
/// Max. number of memory partitions
pub const OS_MAX_MEM_PART: usize = 5;
/// Max. number of tasks in your application, MUST be >= 2
// pub const OS_MAX_TASKS: usize = 20;
// Max. number of event control blocks in your application
pub const OS_MAX_EVENTS: usize = 20;
/// This const val is used to config the size of ARENA.
/// You can set it refer to the number of tasks in your application(OS_MAX_TASKS) and the number of system tasks(OS_N_SYS_TASKS).
pub const OS_ARENA_SIZE: usize = 10240;
/// Ticks per second of the global timebase. Output frequency of the Timer. Frequency of the Systick(run on Timer)
/// the default one tick is 10us
/// 
///
/// This value is specified by the Cargo features "`tick-hz-*`"
pub const TICK_HZ: u64 = tick::TICK_HZ;

lazy_static::lazy_static! {
    /// input frequency of the Timer, you should config it yourself(set the Hardware)
    pub static ref APB_HZ: UPSafeCell<u64> = unsafe {
        UPSafeCell::new(0)
    };
    /// the system clock frequency, you should config it yourself(set the Hardware)
    pub static ref SYSCLK_HZ: UPSafeCell<u64> = unsafe {
        UPSafeCell::new(0)
    };
}

/// the block delay of idle task in poll
#[cfg(feature = "delay_idle")]
pub const block_delay_poll: usize = 2;