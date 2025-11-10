# Embassy Preempt Config

Embassy Preempt RTOS 的配置模块，提供系统常量、类型定义和编译时配置选项。

## 概述

该模块定义了 RTOS 内核所需的所有配置常量、类型别名和系统参数。通过编译时特性标志，可以为不同的应用场景定制系统行为，实现最优的性能和内存使用。

## 核心组件

### 系统常量

#### 基础配置常量

```rust
/// 最高优先级数值（数字越小优先级越高）
pub const OS_LOWEST_PRIO: OS_PRIO = 63;

/// 任务变量表大小
pub const OS_TASK_REG_TBL_SIZE: USIZE = 1;

/// 内存分区最大数量
pub const OS_MAX_MEM_PART: USIZE = 5;

/// 应用中最大任务数
pub const OS_MAX_EVENTS: USIZE = 20;

/// Arena 内存池大小
pub const OS_ARENA_SIZE: USIZE = 10240;
```

#### 优先级相关常量

```rust
/// 空闲任务优先级
pub const OS_TASK_IDLE_PRIO: OS_PRIO = OS_LOWEST_PRIO;

/// 统计任务优先级
pub const OS_TASK_STAT_PRIO: OS_PRIO = OS_LOWEST_PRIO - 1;

/// 自身优先级标志
pub const OS_PRIO_SELF: INT32U = 0xFF;
```

### 基础类型定义

#### 整数类型

```rust
/// 布尔类型
pub type BOOLEAN = bool;

/// 8位无符号整数
pub type INT8U = u8;
/// 8位有符号整数
pub type INT8S = i8;

/// 16位无符号整数
pub type INT16U = u16;
/// 16位有符号整数
pub type INT16S = i16;

/// 32位无符号整数
pub type INT32U = u32;
/// 32位有符号整数
pub type INT32S = i32;

/// 64位无符号整数
pub type INT64U = u64;

/// 指针类型
pub type PTR = *mut ();

/// 数组索引类型
pub type USIZE = usize;
```

#### RTOS 特定类型

```rust
/// 栈条目类型 (32位宽）
pub type OS_STK = usize;

/// CPU 状态寄存器大小 (32位）
pub type OS_CPU_SR = u32;

/// 优先级类型（根据配置确定）
#[cfg(feature = "OS_PRIO_LESS_THAN_64")]
pub type OS_PRIO = u8;

#[cfg(feature = "OS_PRIO_LESS_THAN_256")]
pub type OS_PRIO = INT16U;
```

### 时钟配置

#### Tick 频率配置

通过特性标志选择系统时钟频率：

```rust
// 默认值（如未指定任何 tick-hz 特性）
pub const TICK_HZ: INT64U = 100_000;
```

### 全局状态变量

#### 系统时钟频率

```rust
lazy_static! {
    /// APB 总线频率（需要用户配置）
    pub static ref APB_HZ: UPSafeCell<INT64U> = unsafe {
        UPSafeCell::new(0)
    };

    /// 系统时钟频率（需要用户配置）
    pub static ref SYSCLK_HZ: UPSafeCell<INT64U> = unsafe {
        UPSafeCell::new(0)
    };
}
```

#### 延迟配置

```rust
/// 空闲任务的块延迟时间
#[cfg(feature = "delay_idle")]
pub const block_delay_poll: usize = 2;
```

### ucosii 模块

#### 错误码枚举

```rust
#[derive(PartialEq)]
#[repr(align(4))]
#[repr(C)]
pub enum OS_ERR_STATE {
    OS_ERR_NONE,                    // 无错误
    OS_ERR_EVENT_TYPE,              // 事件类型无效
    OS_ERR_PEND_ISR,                // 任务在 ISR 中挂起
    OS_ERR_TIMEOUT,                  // 操作超时
    // ... 更多错误码
}
```

#### 全局状态变量

```rust
/// 当前系统时间 (以 tick 计）
#[cfg(feature = "OS_TIME_GET_SET_EN")]
pub static OSTime: AtomicU32 = AtomicU32::new(0);

/// 中断嵌套层级
pub static OSIntNesting: AtomicU8 = AtomicU8::new(0);

/// 调度器锁嵌套层级
pub static OSLockNesting: AtomicU8 = AtomicU8::new(0);

/// 已创建任务数量
pub static OSTaskCtr: AtomicU8 = AtomicU8::new(0);

/// 内核运行标志
pub static OSRunning: AtomicBool = AtomicBool::new(false);
```

#### 优先级解析表

```rust
/// 优先级解析表，用于 O(1) 优先级查找
pub const OSUnMapTbl: [INT8U; 256] = [
    0, 0, 1, 0, 2, 0, 1, 0, 3, 0, 1, 0, 2, 0, 1, 0, // 0x00 to 0x0F
    4, 0, 1, 0, 2, 0, 1, 0, 3, 0, 1, 0, 2, 0, 1, 0, // 0x10 to 0x1F
    // ... 完整的256字节表
];
```

## 配置选项

### 优先级范围

```toml
# 支持最多 63 个优先级
features = ["OS_PRIO_LESS_THAN_64"]

# 支持最多 255 个优先级
features = ["OS_PRIO_LESS_THAN_256"]

# 互斥特性，不能同时启用
# 编译时检查防止错误配置
#[cfg(all(feature = "OS_PRIO_LESS_THAN_64", feature = "OS_PRIO_LESS_THAN_256"))]
compile_error!("You may not enable both `OS_PRIO_LESS_THAN_64` and `OS_PRIO_LESS_THAN_256` features.");
```

### 时钟频率

```toml
# 在 Cargo.toml 中选择时钟频率
[dependencies.embassy-preempt-cfg]
features = ["tick-hz-1_000"]  # 1 kHz 时钟频率
```

### 功能特性

```toml
# 启用可选功能
features = [
    "OS_TASK_NAME_EN",     # 任务命名功能
    "OS_EVENT_NAME_EN",    # 事件命名功能
    "delay_idle",          # 空闲任务延迟
    "OS_SCHED_LOCK_EN",     # 调度器锁
]
```