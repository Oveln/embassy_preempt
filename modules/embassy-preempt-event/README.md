# Embassy Preempt Event

Embassy Preempt RTOS 的事件管理模块，提供同步原语的基础框架和部分实现。

## 概述

该模块实现了 RTOS 同步原语的基础框架，目前主要完成了信号量的基本功能。其他同步原语（互斥锁、事件标志、邮箱、消息队列）的框架已经搭建，但具体实现还在开发中。所有事件控制块都来自全局事件池管理。

## 当前实现状态

### ✅ 已实现功能

#### 信号量 (Semaphore)
- **基础实现**: 创建、等待、发布信号量
- **事件控制块**: 完整的事件控制块结构
- **事件池管理**: 全局事件池的分配和释放

```rust
use embassy_preempt_event::os_sem::{OSSemCreate, OSSemPend, OSSemPost};

// 创建信号量
let sem = OSSemCreate(initial_count)?;

// 等待信号量
OSSemPend(sem, timeout)?;

// 发布信号量
OSSemPost(sem);

// 非阻塞尝试获取
let count = OSSemAccept(sem);
```

#### 事件池 (Event Pool)
- **内存管理**: 基于全局 Arena 的内存池
- **分配/释放**: 事件控制块的动态管理
- **链表管理**: 空闲事件控制块的链表组织

```rust
use embassy_preempt_event::{GlobalEventPool, EventPool};

let pool = GlobalEventPool.as_ref().unwrap();

// 从事件池分配事件控制块
let event = pool.alloc()?;

// 释放事件控制块到事件池
pool.free(event);
```

### 🚧 框架已完成

#### 事件控制块结构
- **OS_EVENT**: 完整的事件控制块定义
- **OS_EVENT_TYPE**: 支持所有事件类型（信号量、互斥锁、邮箱、队列、事件标志）
- **OS_EVENT_REF**: 事件控制块的安全引用包装

#### 任务等待管理
- **OS_EventTaskWait**: 将任务加入事件等待列表
- **OS_EventTaskRdy**: 从等待列表唤醒任务
- **OS_EventTaskRemove**: 从等待列表移除任务

### ❌ 待实现功能

#### 异步的信号量

#### 互斥锁 (Mutex)

#### 事件标志组 (Event Flags)

#### 邮箱 (Mailbox)

#### 消息队列 (Queue)

## 核心组件

### 事件控制块 (OS_EVENT)

所有同步原语的基础数据结构：

```rust
pub struct OS_EVENT {
    /// 事件类型
    pub OSEventType: OS_EVENT_TYPE,
    /// 事件相关数据指针
    pub OSEventPtr: SyncUnsafeCell<Option<OS_EVENT_REF>>,
    /// 信号量计数（仅信号量使用）
    pub OSEventCnt: INT16U,
    /// 等待任务组位图
    pub OSEventGrp: OS_PRIO,
    /// 等待任务表位图
    pub OSEventTbl: [OS_PRIO; OS_EVENT_TBL_SIZE as usize],
    /// 事件名称（可选）
    #[cfg(feature = "OS_EVENT_NAME_EN")]
    pub OSEventName: String,
}
```

### 事件类型

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OS_EVENT_TYPE {
    UNUSED = 0,     // 未使用
    MBOX = 1,       // 邮箱
    Q = 2,          // 消息队列
    SEM = 3,        // 信号量
    MUTEX = 4,      // 互斥锁
    FLAG = 5,       // 事件标志组
}
```

### 全局事件池

统一管理所有事件控制块的内存池：

```rust
lazy_static! {
    pub static ref GlobalEventPool: Option<EventPool> = Some(EventPool::new());
}

pub struct EventPool {
    /// 空闲事件控制块链表
    pub OSEventFreeList: SyncUnsafeCell<Option<OS_EVENT_REF>>,
    /// 事件控制块表
    OSEventTbl: SyncUnsafeCell<[OS_EVENT_REF; OS_MAX_EVENTS as usize]>,
}
```

## 同步原语

### 信号量 (Semaphore)

用于资源计数和任务同步。

#### 创建信号量

```rust
use embassy_preempt_event::os_sem::{OSSemCreate, OSSemPend, OSSemPost};

// 创建计数信号量
let sem = OSSemCreate(initial_count)?;

// 创建二进制信号量
let binary_sem = OSSemCreate(1)?;

// 创建互斥信号量
let mutex_sem = OSSemCreate(0)?;
```

#### 等待和释放信号量

```rust
// 等待信号量（带超时）
let result = OSSemPend(sem, timeout_ticks);
match result {
    OS_ERR_STATE::OS_ERR_NONE => {
        // 成功获取信号量
    }
    OS_ERR_STATE::OS_ERR_TIMEOUT => {
        // 等待超时
    }
    OS_ERR_STATE::OS_ERR_PEND_ISR => {
        // 不能在中断中等待
    }
}

// 释放信号量
OSSemPost(sem);
```

#### 非阻塞操作

```rust
use embassy_preempt_event::os_sem::OSSemAccept;

// 非阻塞获取信号量
let cnt = OSSemAccept(sem);
if cnt > 0 {
    // 成功获取信号量
} else {
    // 信号量不可用
}
```

### 互斥锁 (Mutex)

提供互斥访问保护，支持优先级继承。

#### 创建互斥锁

```rust
use embassy_preempt_event::os_mutex::{OSMutexCreate, OSMutexPend, OSMutexPost};

// 创建互斥锁
let mutex = OSMutexCreate()?;
```

#### 获取和释放互斥锁

```rust
// 获取互斥锁
OSMutexPend(mutex, timeout_ticks)?;

// 临界区代码
// ... 共享资源访问 ...

// 释放互斥锁
OSMutexPost(mutex);
```

### 事件标志组 (Event Flags)

用于多个事件的组合等待。

#### 创建事件标志组

```rust
use embassy_preempt_event::os_flag::{OSFlagCreate, OSFlagPend, OSFlagPost};

// 创建事件标志组
let flags = OSFlagCreate(0)?;
```

#### 等待事件

```rust
use embassy_preempt_event::os_flag::{
    OS_FLAG_WAIT_SET_ALL, OS_FLAG_WAIT_SET_ANY,
    OS_FLAG_CONSUME
};

// 等待所有指定标志位被设置
OSFlagPend(
    flags,
    0x0F,           // 等待低4位被设置
    OS_FLAG_WAIT_SET_ALL,
    timeout,
    OS_FLAG_CONSUME
)?;

// 等待任意指定标志位被设置
OSFlagPend(
    flags,
    0x0F,           // 等待低4位中任意一位被设置
    OS_FLAG_WAIT_SET_ANY,
    timeout,
    0               // 不消费标志
)?;
```

#### 设置和清除事件标志

```rust
// 设置事件标志
OSFlagPost(flags, 0x0F, OS_FLAG_SET)?;

// 清除事件标志
OSFlagPost(flags, 0x0F, OS_FLAG_CLR)?;
```

### 邮箱 (Mailbox)

用于单消息传递。

#### 创建邮箱

```rust
use embassy_preempt_event::os_mbox::{OSMboxCreate, OSMboxPend, OSMboxPost};

// 创建邮箱（初始消息可选）
let mbox = OSMboxCreate(initial_message)?;
```

#### 发送和接收消息

```rust
// 发送消息到邮箱
OSMboxPost(mbox, message_ptr, timeout)?;

// 从邮箱接收消息
let message = OSMboxPend(mbox, timeout)?;

// 非阻塞尝试接收
let message = OSMboxAccept(mbox);
```

### 消息队列 (Queue)

用于多消息的FIFO传递。

#### 创建消息队列

```rust
use embassy_preempt_event::os_q::{OSQCreate, OSQPost, OSQPend};

let message_storage: [PTR; 10] = [ptr::null_mut(); 10];
let queue = OSQCreate(message_storage.as_ptr(), 10)?;
```

#### 发送和接收消息

```rust
// 发送消息到队列
OSQPost(queue, message_ptr)?;

// 发送到队列前端（高优先级）
OSQPostFront(queue, message_ptr)?;

// 从队列接收消息
let message = OSQPend(queue, timeout)?;

// 非阻塞尝试接收
let message = OSQAccept(queue)?;
```

## 事件等待管理

### 等待列表操作

```rust
use embassy_preempt_event::{
    OS_EventTaskWait, OS_EventTaskRdy, OS_EventTaskRemove
};

// 将任务加入事件等待列表
OS_EventTaskWait(event);

// 将任务从等待列表移除并使其就绪
OS_EventTaskRdy(event);

// 将任务从等待列表移除（不解锁）
OS_EventTaskRemove(task, event);
```

### 事件池管理

```rust
use embassy_preempt_event::GlobalEventPool;

// 从事件池分配事件控制块
let pool = GlobalEventPool.as_ref().unwrap();
let event = pool.alloc()?;

// 释放事件控制块到事件池
pool.free(event);
```

## 配置选项

### 功能特性

```toml
[dependencies.embassy-preempt-event]
features = [
    "OS_EVENT_NAME_EN",    # 启用事件命名
    "OS_SEM_EN",          # 启用信号量
    "OS_MUTEX_EN",        # 启用互斥锁
    "OS_FLAG_EN",         # 启用事件标志
    "OS_MBOX_EN",         # 启用邮箱
    "OS_Q_EN",            # 启用消息队列
]
```

### 调试功能

```toml
features = [
    "OS_DEBUG_EN",        # 启用调试功能
    "OS_ARG_CHK_EN",      # 启用参数检查
]
```

## 使用示例

### 信号量基本使用

```rust
use embassy_preempt_event::os_sem::{OSSemCreate, OSSemPend, OSSemPost};

// 创建计数信号量
let semaphore = OSSemCreate(1)?;

// 在任务中使用信号量
fn access_resource(sem: OS_EVENT_REF) {
    // 等待信号量
    if OSSemPend(sem, 1000).is_ok() {
        // 成功获取信号量，访问资源
        access_shared_data();

        // 释放信号量
        OSSemPost(sem);
    } else {
        // 等待超时
        handle_timeout();
    }
}
```

### 事件池管理

```rust
use embassy_preempt_event::{GlobalEventPool, OS_EVENT_TYPE};

// 获取全局事件池
let pool = GlobalEventPool.as_ref().unwrap();

// 分配事件控制块
if let Some(event) = pool.alloc() {
    // 配置事件类型
    event.OSEventType = OS_EVENT_TYPE::SEM;
    event.OSEventCnt = 1;

    // 使用事件控制块...

    // 释放回事件池
    pool.free(event);
} else {
    // 事件池已满
    handle_pool_full();
}
```

## 开发状态

### 当前限制

1. **功能不完整**: 大部分同步原语仅有框架，具体实现待开发
2. **特性控制**: 部分功能被特性标志注释掉 (`#[cfg(feature = "OS_EVENT_EN")]`)
3. **文档缺失**: 待实现功能缺少详细的文档和使用示例

### 开发计划

1. **完善信号量**: 添加优先级继承、错误处理等高级功能
2. **实现互斥锁**: 完成互斥锁的基本功能和优先级继承
3. **实现事件标志**: 完成事件标志组的位操作和等待机制
4. **实现邮箱**: 完成单消息传递机制
5. **实现消息队列**: 完成FIFO消息队列机制

### 中断处理

```rust
// 当前的中断安全检查
use embassy_preempt_cfg::ucosii::OSIntNesting;

// 在创建信号量时检查中断上下文
if OSIntNesting.load(Ordering::Acquire) > 0 {
    return None;  // 不能在中断中创建信号量
}

// 发布操作支持在中断中使用
OSSemPost(semaphore);  // 中断安全
```

## 性能特性

### 时间复杂度

- **事件创建**: O(1) - 从预分配池中获取
- **事件等待**: O(1) - 直接位图操作
- **事件发布**: O(1) - 直接位图操作
- **队列操作**: O(1) - 循环缓冲区

### 空间复杂度

- **事件控制块**: 固定大小结构体
- **等待列表**: O(n) 位图空间，n为优先级数
- **消息队列**: 用户指定的缓冲区空间

### 实时性保证

- **优先级继承**: 互斥锁支持优先级继承避免优先级反转
- **立即唤醒**: 发布操作立即触发调度器检查
- **最小等待**: 优化的位图查找算法

## 与其他模块的集成

### executor 模块

```rust
// 使用全局执行器进行任务调度
use embassy_preempt_executor::GlobalSyncExecutor;

let executor = GlobalSyncExecutor.as_ref().unwrap();
executor.enqueue(task);
```

### cfg 模块

```rust
// 使用配置模块的类型和常量
use embassy_preempt_cfg::{
    OS_MAX_EVENTS, OS_EVENT_TBL_SIZE, INT16U
};
```

### log 模块

```rust
// 使用日志模块记录事件操作
use embassy_preempt_log::task_log;

task_log!(info, "Semaphore {} created", sem_id);
```

## 调试和诊断

### 事件状态查询

```rust
// 查询信号量状态
use embassy_preempt_event::os_sem::OSSemQuery;

let sem_data = OSSemQuery(semaphore)?;
println!("Available permits: {}", sem_data.OSCnt);

// 查询互斥锁状态
use embassy_preempt_event::os_mutex::OSMutexQuery;

let mutex_data = OSMutexQuery(mutex)?;
println!("Owner priority: {}", mutex_data.OSOwnerPrio);
```

### 事件名称

```rust
#[cfg(feature = "OS_EVENT_NAME_EN")]
// 设置事件名称
use embassy_preempt_event::os_sem::OSSemNameSet;

OSSemNameSet(semaphore, "ResourceSemaphore")?;

// 获取事件名称
use embassy_preempt_event::os_sem::OSSemNameGet;

let name = OSSemNameGet(semaphore)?;
```

## 最佳实践

### 1. 选择合适的同步原语

- **信号量**: 资源计数、简单同步
- **互斥锁**: 互斥访问保护、支持优先级继承
- **事件标志**: 多个事件的组合等待
- **邮箱**: 单消息传递
- **消息队列**: 多消息FIFO传递

### 2. 避免死锁

```rust
// 按固定顺序获取多个互斥锁
OSMutexPend(mutex_a, timeout)?;
OSMutexPend(mutex_b, timeout)?;

// 使用资源

// 按相反顺序释放
OSMutexPost(mutex_b);
OSMutexPost(mutex_a);
```

### 3. 合理设置超时

```rust
// 避免无限等待
const OS_NO_TIMEOUT: u32 = 0;
const INFINITE_TIMEOUT: u32 = u32::MAX;

// 推荐使用合理超时
let timeout = 1000; // 1秒超时
```

### 4. 中断安全操作

```rust
// 在ISR中只使用发布操作
#[interrupt]
fn data_ready_interrupt() {
    OSMboxPost(mailbox, data_ptr);
    OSSemPost(data_ready_sem);
}

// 不要在ISR中使用等待操作
// 错误：OSSemPend(sem, timeout); // 不能在ISR中使用
```

## 注意事项

1. **中断安全**: 等待操作不能在中断服务程序中使用
2. **优先级反转**: 使用互斥锁时注意优先级继承
3. **内存限制**: 事件控制块数量有限制
4. **超时处理**: 合理设置超时避免无限等待
5. **资源清理**: 及时释放不再使用的事件控制块

## 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。