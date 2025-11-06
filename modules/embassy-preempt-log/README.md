# Embassy Preempt Log

为 Embassy Preempt RTOS 设计的日志记录 crate，提供基于功能标志的可选日志级别的专用日志宏，包装 `defmt` 实现。

## 功能特性

- **log-base**: 基于 defmt 的基础日志功能
- **log-os**: 操作系统级日志
- **log-task**: 任务相关日志
- **log-mem**: 内存管理日志
- **log-timer**: 定时器相关日志
- **log-scheduler**: 调度器日志

## 使用方法

在您的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
embassy-preempt-log = { path = "../modules/embassy-preempt-log", features = ["log-os", "log-task"] }
```

### 示例

#### 基本用法
```rust
// 导入专用的日志宏
use embassy_preempt_log::{os_log, task_log, scheduler_log, timer_log, mem_log};

fn os_init() {
    os_log!(info, "操作系统初始化成功");
    os_log!(debug, "内存布局: {:?}", memory_layout);
    os_log!(warn, "检测到高CPU使用率: {}%", cpu_usage);
    os_log!(error, "初始化子系统失败: {:?}", error);
}

fn task_management() {
    task_log!(info, "创建优先级为 {} 的新任务", priority);
    task_log!(debug, "任务栈大小: {} 字节", stack_size);
    task_log!(trace, "任务上下文切换至 {:?}", next_task);
    task_log!(warn, "任务 {} 超出了其时间片", task_id);
}

fn scheduler_operations() {
    scheduler_log!(info, "调度器启动，有 {} 个就绪任务", ready_count);
    scheduler_log!(debug, "当前任务: {:?}, 下一个任务: {:?}", current, next);
    scheduler_log!(trace, "定时器队列状态: {:?}", timer_queue);
}

fn timer_management() {
    timer_log!(info, "为任务 {} 创建延迟 {}ms 的定时器", task_id, delay);
    timer_log!(debug, "定时器滴答: {}, 下次到期: {}", tick_count, next_expiry);
    timer_log!(warn, "检测到定时器溢出，重置计数器");
}

fn memory_operations() {
    mem_log!(info, "堆初始化完成: 总计 {} 字节", heap_size);
    mem_log!(debug, "从堆分配 {} 字节", size);
    mem_log!(trace, "空闲块: {}, 已使用块: {}", free_count, used_count);
    mem_log!(warn, "内存碎片: {}%", fragmentation_percent);
    mem_log!(error, "内存不足！无法分配 {} 字节", size);
}
```

#### 功能标志组合
```toml
# 仅启用基本日志（操作系统和任务）
[dependencies]
embassy-preempt-log = { path = "../modules/embassy-preempt-log", features = ["log-os", "log-task"] }

# 启用所有日志功能
[dependencies]
embassy-preempt-log = { path = "../modules/embassy-preempt-log", features = [
    "log-os", "log-task", "log-mem", "log-timer", "log-scheduler"
] }

# 生产环境禁用所有日志（零成本）
[dependencies]
embassy-preempt-log = { path = "../modules/embassy-preempt-log" }
```

#### 条件编译
```rust
// 仅在启用 log-os 功能时编译
#[cfg(feature = "log-os")]
fn detailed_os_init() {
    os_log!(debug, "详细的操作系统初始化信息");
    // ... 详细初始化代码 ...
}

// 当功能禁用时，日志变成空操作
fn performance_critical_function() {
    os_log!(trace, "进入性能关键函数");
    // ... 性能关键代码 ...
    os_log!(trace, "退出性能关键函数");
    // 当 log-os 禁用时，这些调用编译为空
}
```

## 设计原则

此 crate 遵循以下原则：

1. **零成本抽象**: 当日志功能被禁用时，所有日志宏在编译时变成空操作
2. **模块化设计**: 每个 RTOS 组件都有自己的日志命名空间，便于组织管理
3. **基于功能的控制**: 通过功能标志对日志内容进行精细控制
4. **不导出基础宏**: 不直接导出基础日志宏（info!, debug! 等），避免命名空间污染

## 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。