// TODO: Make all the config to be feature!!!

/// the const val define the lowest prio
pub const OS_LOWEST_PRIO: u8 = 63;
/// Size of task variables array (#of u32 entries)
pub const OS_TASK_REG_TBL_SIZE: usize = 1;
/// Max. number of memory partitions
pub const OS_MAX_MEM_PART: usize = 5;
/// Max. number of tasks in your application, MUST be >= 2
// pub const OS_MAX_TASKS: usize = 20;
/// This const val is used to config the size of ARENA.
/// You can set it refer to the number of tasks in your application(OS_MAX_TASKS) and the number of system tasks(OS_N_SYS_TASKS).
pub const OS_ARENA_SIZE: usize = 10240;
/// output frequency of the Timer. frequency of the Systick(run on Timer)
/// one tick is 10us
pub const TICK_HZ: u64 = 100_000;
/// input frequency of the Timer, you should config it yourself(set the Hardware)
/// STM32F401RE APB1 frequency: 84MHz
pub const APB_HZ: u64 = 84000000;
/// the block delay of idle task in poll
#[cfg(feature = "delay_idle")]
pub const block_delay_poll: usize = 2;