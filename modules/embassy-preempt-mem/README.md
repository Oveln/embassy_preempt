# Embassy Preempt Mem

为 Embassy Preempt RTOS 提供内存管理功能的核心模块。

## 概述

该模块为 RTOS 内核提供完整的内存管理解决方案，包括内存分配器、堆管理、固定大小块分配器等功能。所有组件都经过专门优化，适用于嵌入式实时系统的性能和内存要求。

## 核心组件

### arena 模块

提供内存竞技场（arena）分配器，用于高效的内存管理。

### heap 模块

提供多种堆分配策略和内存分配功能。

#### 主要功能

- **linked_list**: 基于链表的堆管理
- **fixed_size_block**: 固定大小块分配器
- **stack_allocator**: 栈式分配器

#### 使用示例

```rust
use embassy_preempt_mem::heap::{alloc_stack, OS_STK_REF, TASK_STACK_SIZE};
use core::alloc::Layout;

// 分配任务栈
let layout = Layout::from_size_align(TASK_STACK_SIZE, 4).unwrap();
let stack_ref = alloc_stack(layout);

// 使用栈引用
println!("栈底地址: {:?}", stack_ref.STK_REF);
```

### 内存管理特性

1. **高性能**: 针对嵌入式系统优化的内存分配算法
2. **确定性**: 实时系统所需的可预测内存分配时间
3. **低开销**: 最小化内存碎片和分配开销
4. **多种策略**: 支持不同的内存分配策略以适应不同场景

## 依赖关系

- `embassy-preempt-structs`: 基础数据结构
- `embassy-preempt-cfg`: 配置管理
- `embassy-preempt-log`: 日志功能

## 特性标志

- `OS_MEM_EN`: 启用内存管理功能
- `log-mem`: 启用内存管理相关的日志记录

## 集成

该模块被 `embassy-preempt-executor` 使用，为任务栈分配和内存管理提供支持。通过模块化设计，可以独立测试和优化内存管理功能。