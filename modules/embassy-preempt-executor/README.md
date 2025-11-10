# Embassy Preempt Executor

Embassy Preempt RTOS 的核心执行器和调度器模块，实现混合任务模型和智能栈管理。

## 概述

该模块是整个 RTOS 的核心，负责任务调度、内存管理、定时器系统以及异步任务的执行。它创新性地结合了同步任务和异步任务的混合执行模型，实现了智能的栈分配策略。

## 核心特性

### 混合任务模型

- **异步任务 (AsyncTask)**: 基于 Rust Future 的协程模型，高效利用栈空间
- **同步任务 (SyncTask)**: 传统线程模型，每个任务有独立的执行上下文
- **统一调度**: 两种任务类型在同一个调度器中统一管理

### 智能栈管理

- **栈复用**: 主动让权的任务共享栈空间，最大化内存效率
- **独立栈**: 被抢占的任务分配独立栈，保证实时性要求
- **自适应分配**: 根据调度模式动态选择最优的内存管理策略

### 高效调度算法

- **O(1) 调度**: 基于优先级位图的 O(1) 最高优先级任务查找
- **抢占式内核**: 支持中断上下文中的立即任务切换
- **优先级管理**: 支持最多 64 个优先级级别

## 模块结构

### 核心组件

#### SyncExecutor

全局同步执行器，负责任务调度和上下文切换：

```rust
pub struct SyncExecutor {
    // 优先级表：优先级到任务控制块的映射
    os_prio_tbl: SyncUnsafeCell<[OS_TCB_REF; OS_LOWEST_PRIO + 1]>,

    // 当前运行任务状态
    OSPrioCur: SyncUnsafeCell<OS_PRIO>,
    OSTCBCur: SyncUnsafeCell<OS_TCB_REF>,

    // 最高优先级就绪任务
    OSPrioHighRdy: SyncUnsafeCell<OS_PRIO>,
    OSTCBHighRdy: SyncUnsafeCell<OS_TCB_REF>,

    // 就绪队列位图
    OSRdyGrp: SyncUnsafeCell<u8>,
    OSRdyTbl: SyncUnsafeCell<[u8; OS_RDY_TBL_SIZE]>,

    // 定时器队列
    timer_queue: timer_queue::TimerQueue,
    alarm: AlarmHandle,
}
```

#### 任务控制块 (OS_TCB)

任务的核心数据结构：

```rust
pub struct OS_TCB {
    // 栈管理
    pub(crate) OSTCBStkPtr: Option<OS_STK_REF>,
    pub(crate) needs_stack_save: SyncUnsafeCell<bool>,

    // 调度信息
    pub(crate) OSTCBPrio: INT8U,
    pub(crate) OSTCBStat: State,
    pub(crate) expires_at: SyncUnsafeCell<u64>,

    // 优先级位图优化
    pub OSTCBX: INT8U,    // 组内位位置
    pub OSTCBY: INT8U,    // 组索引
    pub(crate) OSTCBBitX: INT8U,
    pub(crate) OSTCBBitY: INT8U,

    // 异步任务支持
    pub(crate) OS_POLL_FN: SyncUnsafeCell<Option<unsafe fn(OS_TCB_REF)>>,
}
```

#### 内存管理模块

##### Arena

基于固定大小内存池的分配器：

```rust
// 全局内存池
lazy_static! {
    pub static ref ARENA: Option<Arena> = Some(Arena::new());
}

pub struct Arena {
    // 固定大小的内存块
    memory: SyncUnsafeCell<[u8; OS_ARENA_SIZE]>,
    // 位图管理空闲块
    bitmap: SyncUnsafeCell<[usize; BITMAP_SIZE]>,
}
```

##### Heap

动态堆内存管理器：

```rust
pub struct Heap {
    // 链表分配器
    linked_list: LinkedListAllocator,
    // 固定大小块分配器
    fixed_block: FixedSizeBlockAllocator,
    // 栈分配器
    stack_allocator: StackAllocator,
}
```

### 定时器系统

#### TimerQueue

基于最小堆的定时器队列：

```rust
pub struct TimerQueue {
    // 最小堆存储定时器
    heap: SyncUnsafeCell<Vec<TimerEntry>>,
    // 当前时间基准
    set_time: SyncUnsafeCell<u64>,
}
```

## 任务创建和管理

### 异步任务创建

```rust
use embassy_preempt_executor::AsyncOSTaskCreate;

// 创建异步任务
AsyncOSTaskCreate(
    my_async_task,           // 异步函数
    ptr::null_mut(),         // 参数
    ptr::null_mut(),         // 栈指针 (可选)
    priority,                // 优先级
);

// 异步任务定义
async fn my_async_task() {
    loop {
        // 异步操作
        embassy_preempt_log::info!("Async task running");
        embassy_preempt::time::delay_ms(1000).await;
    }
}
```

### 同步任务创建

```rust
use embassy_preempt_executor::SyncOSTaskCreate;

// 创建同步任务
SyncOSTaskCreate(
    my_sync_task,            // 同步函数
    ptr::null_mut(),         // 参数
    ptr::null_mut(),         // 栈指针 (可选)
    priority,                // 优先级
);

// 同步任务定义
extern "C" fn my_sync_task(_arg: *mut c_void) -> ! {
    loop {
        // 同步操作
        embassy_preempt_log::info!("Sync task running");
        embassy_preempt::time::delay_ms(1000);
    }
}
```

## 调度算法

### 优先级位图

使用位图算法实现 O(1) 时间的最高优先级任务查找：

```rust
impl SyncExecutor {
    // 查找最高优先级就绪任务
    pub fn find_highrdy_prio(&self) -> OS_PRIO {
        let tmp = self.OSRdyGrp.get_unmut();
        if *tmp == 0 {
            return OS_TASK_IDLE_PRIO;
        }

        // 使用查表法 O(1) 找到最高优先级
        let y = OSUnMapTbl[*tmp as usize];
        let x = OSUnMapTbl[self.OSRdyTbl.get_unmut()[y as usize] as usize];
        (y << 3) + x
    }
}
```

### 上下文切换

#### 中断上下文切换

```rust
// 在中断中触发的上下文切换
pub unsafe fn IntCtxSW(&'static self) {
    critical_section::with(|_| {
        let new_prio = self.find_highrdy_prio();
        if new_prio < self.OSPrioCur.get() &&
           OSIntNesting.load(Ordering::Acquire) == 0 &&
           OSLockNesting.load(Ordering::Acquire) == 0 {
            self.set_highrdy_with_prio(new_prio);
            self.interrupt_poll();
        }
    });
}
```

#### 任务级上下文切换

```rust
// 在任务级触发的上下文切换
pub unsafe fn poll(&'static self) -> ! {
    loop {
        let task = critical_section::with(|_| {
            let task = self.OSTCBHighRdy.get();
            if task.OSTCBStkPtr.is_none() {
                // 异步任务，直接执行
                self.single_poll(task);
                None
            } else {
                // 同步任务，恢复上下文
                task.restore_context_from_stk();
                Some(task)
            }
        });

        if task.is_none() {
            continue;
        }
        // 对于同步任务，执行到这里不会返回
    }
}
```

## 内存管理策略

### 栈分配

```rust
// 根据任务类型分配栈
fn allocate_stack_for_task(task: OS_TCB_REF) -> OS_STK_REF {
    if *self.OSPrioCur.get_unmut() == OS_TASK_IDLE_PRIO {
        // 空闲任务使用程序栈
        program_stk.clone()
    } else {
        // 普通任务分配新栈
        let layout = Layout::from_size_align(TASK_STACK_SIZE, 4).unwrap();
        alloc_stack(layout)
    }
}
```

### 事件池管理

```rust
// 从 Arena 分配任务控制块
impl EventPool {
    fn claim(cs: CriticalSection) -> OS_EVENT_REF {
        let event = ARENA.alloc::<OS_EVENT>(cs);
        event.write(OS_EVENT::new());

        OS_EVENT_REF {
            ptr: Some(NonNull::new(event as *mut _ as _).unwrap())
        }
    }
}
```

## 定时器系统

### 定时器回调

```rust
// 定时器中断回调处理
fn alarm_callback(ctx: *mut ()) {
    let this: &Self = unsafe { &*(ctx as *const Self) };

    loop {
        // 处理所有到期的定时器
        this.timer_queue.dequeue_expired(RTC_DRIVER.now(), wake_task_no_pend);

        // 设置下一次定时器
        let next_expire = this.timer_queue.next_expiration();
        this.timer_queue.set_time.set(next_expire);

        if RTC_DRIVER.set_alarm(this.alarm, next_expire) {
            break;
        }
    }

    // 触发上下文切换
    unsafe { this.IntCtxSW() };
}
```

## 初始化和启动

### 系统初始化

```rust
use embassy_preempt_executor::{OSInit, OSStart};

fn main() -> ! {
    // 初始化系统
    OSInit();

    // 创建任务
    AsyncOSTaskCreate(task1, ptr::null_mut(), ptr::null_mut(), 10);
    SyncOSTaskCreate(task2, ptr::null_mut(), ptr::null_mut(), 5);

    // 启动调度器 (永不返回)
    OSStart();
}
```

### OSInit 流程

1. 初始化内存管理器 (Heap, StackAllocator, Arena)
2. 初始化片上外设
3. 创建Idle task (协程)
4. 初始化时钟 (开始systick中断)
5. 初始化事件队列

## 性能特性

### 时间复杂度

- **任务调度**: O(1) - 基于优先级位图
- **任务入队**: O(1) - 直接位图操作
- **定时器操作**: O(log n) - 基于最小堆
- **内存分配**: O(1) - 固定大小块

### 空间复杂度

- **任务控制块**: 固定大小 + 可选扩展信息
- **就绪队列**: 位图 + 指针数组 (O(n) 空间)
- **定时器队列**: 基于任务数量的动态数组
- **内存池**: 固定大小的连续内存块

### 实时性保证

- **中断延迟**: 最小化中断关闭时间
- **调度延迟**: O(1) 调度决策时间
- **上下文切换**: 优化的栈管理和寄存器保存

## 与其他模块的集成

### event 模块

```rust
// 事件管理使用执行器的任务管理功能
use embassy_preempt_executor::{GlobalSyncExecutor, wake_task};

pub fn OS_EventTaskRdy(pevent: OS_EVENT_REF) {
    let executor = GlobalSyncExecutor.as_ref().unwrap();
    executor.enqueue(task);
}
```

### cfg 模块

```rust
// 使用配置模块的常量和类型
use embassy_preempt_cfg::{
    OS_LOWEST_PRIO, OS_TASK_IDLE_PRIO, OS_RDY_TBL_SIZE
};
```

### log 模块

```rust
// 在调度器中使用日志宏
use embassy_preempt_log::{scheduler_log, task_log};

scheduler_log!(trace, "Context switch: {} -> {}", from, to);
task_log!(info, "Task {} started", task_id);
```

## 调试和诊断

### 任务状态查询

```rust
// 打印就绪队列状态
pub fn print_ready_queue(&self) {
    let tmp: [u8; OS_RDY_TBL_SIZE];
    unsafe {
        tmp = self.OSRdyTbl.get();
    }

    task_log!(info, "Ready queue status:");
    for i in 0..OS_LOWEST_PRIO + 1 {
        if tmp[(i / 8) as usize] & (1 << (i % 8)) != 0 {
            task_log!(info, "Task {} is ready", i);
        }
    }
}
```

### 性能计数器

```rust
// 上下文切换计数
pub fn get_context_switch_count(&self) -> u32 {
    OSCtxSwCtr.load(Ordering::Acquire)
}

// CPU 使用率统计
pub fn get_cpu_usage(&self) -> f32 {
    let idle_time = OSIdleCtr.load(Ordering::Acquire);
    let total_time = OSTime.load(Ordering::Acquire);
    1.0 - (idle_time as f32 / total_time as f32)
}
```

## 注意事项

1. **中断安全**: 确保在中断上下文中使用正确的 API
2. **栈溢出**: 监控任务栈使用，防止栈溢出
3. **优先级反转**: 合理设计任务优先级，避免优先级反转
4. **内存碎片**: 监控堆内存使用，及时清理内存碎片

## 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。