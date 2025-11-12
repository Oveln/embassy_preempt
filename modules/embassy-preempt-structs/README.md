# Embassy Preempt Structs

为 Embassy Preempt RTOS 提供基础数据结构和同步原语的核心模块。

## 概述

该模块提供了 RTOS 内核所需的基础数据结构和同步原语，特别是在单处理器环境下的内部可变性原语。所有结构都经过专门优化，适用于嵌入式实时系统的性能和内存要求。

## 核心组件

### cell 模块

提供内部可变性原语，用于在单处理器环境下安全地共享可变数据。

#### UPSafeCell

单处理器安全单元格，提供内部可变性而无需使用 `unsafe` 代码。

```rust
use embassy_preempt_structs::cell::UPSafeCell;

let safe_cell = UPSafeCell::new(42);

// 获取可变引用
{
    let mut data = safe_cell.exclusive_access();
    *data += 1;
} // 借用在此处结束

// 获取不可变引用
let value = safe_cell.get(); // 返回 Ref<'_, T>

// 设置新值
safe_cell.set(100);

// 交换值并返回旧值
let old_value = safe_cell.swap(200); // old_value = 100
```

#### SyncUnsafeCell

提供同步安全的 UnsafeCell，用于多线程/中断环境下的内部可变性。

```rust
use embassy_preempt_structs::cell::SyncUnsafeCell;
use core::ptr::NonNull;

let safe_cell = SyncUnsafeCell::new(Some(NonNull::dangling()));

// 获取可变指针（unsafe）
let ptr = safe_cell.get();

// 设置值
safe_cell.set(Some(new_value));
```

#### UninitCell

用于管理未初始化数据的单元格，延迟初始化模式。

```rust
use embassy_preempt_structs::cell::UninitCell;

let uninit_cell = UninitCell::uninit();

// 稍后初始化
uninit_cell.write(initial_value);

// 读取值（确保已初始化）
let value = unsafe { uninit_cell.assume_init_read() };
```

## 设计原理

### 单处理器优化

所有原语都针对单处理器环境进行了优化，避免了多处理器环境下的复杂同步开销：

- **UPSafeCell**: 基于 `RefCell` 实现，提供编译时借用检查
- **零成本抽象**: 在 Release 模式下不产生额外运行时开销
- **内存效率**: 紧凑的内存布局，适合资源受限的嵌入式环境

### 线程安全

虽然设计为单处理器使用，但仍确保在中断上下文中的安全性：

- `UPSafeCell`: 通过借用检查防止数据竞争
- `SyncUnsafeCell`: 通过 `Sync` trait 确保跨线程安全
- 中断安全的关键部分保护

### 兼容性

- **no_std**: 完全支持裸机环境
- **外部依赖**: 仅依赖标准库的 `core` 模块
- **Rust 稳定特性**: 不使用实验性特性

## 使用场景

### 任务状态管理

```rust
use embassy_preempt_structs::cell::UPSafeCell;

struct TaskManager {
    task_count: UPSafeCell<u32>,
    current_task: UPSafeCell<Option<TaskId>>,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            task_count: UPSafeCell::new(0),
            current_task: UPSafeCell::new(None),
        }
    }

    fn add_task(&mut self, task_id: TaskId) {
        *self.task_count.exclusive_access() += 1;
    }

    fn set_current_task(&self, task_id: TaskId) {
        self.current_task.set(Some(task_id));
    }
}
```

### 系统状态共享

```rust
use embassy_preempt_structs::cell::SyncUnsafeCell;

static SYSTEM_STATE: SyncUnsafeCell<SystemState> = SyncUnsafeCell::new(SystemState::new());

fn update_system_state() {
    let state = unsafe { &mut *SYSTEM_STATE.get() };
    state.tick_count += 1;
    state.last_update = get_current_time();
}
```

### 延迟初始化

```rust
use embassy_preempt_structs::cell::UninitCell;

static EXPENSIVE_RESOURCE: UninitCell<ExpensiveResource> = UninitCell::uninit();

fn get_resource() -> &'static ExpensiveResource {
    if !EXPENSIVE_RESOURCE.is_init() {
        EXPENSIVE_RESOURCE.write(ExpensiveResource::new());
    }
    unsafe { EXPENSITE_RESOURCE.assume_init_ref() }
}
```

## 性能特性

### 内存使用

- **UPSafeCell**: 仅包含 `RefCell<T>`，无额外开销
- **SyncUnsafeCell**: 直接内存访问，零抽象开销
- **UninitCell**: 延迟初始化，减少内存占用

### 运行时性能

- **借用检查**: 仅在调试模式下进行检查
- **内联优化**: 所有小函数都会被内联
- **无锁设计**: 避免原子操作的开销

### 编译时优化

- **单态化**: 泛型实例化后的特化
- **死代码消除**: 未使用的代码被完全移除
- **链接时优化**: 跨模块的优化

## 与其他模块的集成

### executor 模块

```rust
// 在 executor 中使用 UPSafeCell 管理全局状态
pub struct SyncExecutor {
    os_prio_tbl: SyncUnsafeCell<[OS_TCB_REF; MAX_TASKS]>,
    OSPrioCur: SyncUnsafeCell<OS_PRIO>,
    // ...
}
```

### event 模块

```rust
// 在事件管理中使用 SyncUnsafeCell
pub struct OS_EVENT {
    pub OSEventPtr: SyncUnsafeCell<Option<OS_EVENT_REF>>,
    // ...
}
```

### cfg 模块

```rust
// 配置模块中使用 UPSafeCell
lazy_static! {
    pub static ref APB_HZ: UPSafeCell<INT64U> = unsafe {
        UPSafeCell::new(0)
    };
}
```

## 最佳实践

### 1. 选择合适的原语

- **UPSafeCell**: 单处理器环境下的常规使用
- **SyncUnsafeCell**: 需要在中断上下文中访问的数据
- **UninitCell**: 延迟初始化的资源

### 2. 避免借用冲突

```rust
// ❌ 错误：会导致运行时 panic
let mut borrow1 = safe_cell.exclusive_access();
let mut borrow2 = safe_cell.exclusive_access(); // panic!

// ✅ 正确：使用作用域限制借用
{
    let mut data = safe_cell.exclusive_access();
    // 使用 data
} // 借用结束

let data2 = safe_cell.exclusive_access(); // 安全
```

### 3. 初始化安全性

```rust
// 对于 UninitCell，确保在使用前已初始化
let uninit = UninitCell::uninit();

// 初始化
uninit.write(value);

// 安全使用（确保已初始化）
let value = unsafe { uninit.assume_init_read() };
```

## 注意事项

1. **单处理器限制**: `UPSafeCell` 仅适用于单处理器环境
2. **借用检查**: 运行时借用检查可能导致 panic
3. **UninitCell**: 使用未初始化数据是 unsafe 操作
4. **中断安全**: 在中断上下文中使用时需要额外的小心

## 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。