# Embassy Preempt RTOS 示例项目

本目录包含了 Embassy Preempt RTOS 的实际使用示例，展示了如何在实际项目中应用该实时操作系统。

## 项目结构

```
exam/
├── src/
│   └── bin/
│       ├── hardware_test.rs      # 硬件基础测试
│       ├── preempt_test.rs       # 基础抢占调度测试
│       ├── comprehensive_test.rs # 综合功能测试
│       ├── switch_test.rs        # 异步任务切换示例
│       ├── usart.rs              # 串口通信基础示例
│       ├── usart_test.rs         # 串口通信测试
│       ├── prio_test.rs          # 优先级测试
│       ├── scheduling1_test.rs   # 调度算法测试1
│       ├── scheduling2_test.rs   # 调度算法测试2
│       ├── bottom_test.rs        # 底层功能测试
│       ├── sync_bottom_test.rs   # 同步底层测试
│       ├── time_performance.rs   # 时间性能测试
│       ├── sync_time_performance.rs # 同步时间性能测试
│       ├── space_performance.rs  # 空间性能测试
│       ├── stack_cost_test.rs    # 栈开销测试
│       └── iic_test.rs           # I2C通信测试
├── .cargo/
│   └── config.toml               # Cargo配置
├── Cargo.toml                   # 项目配置
├── build.rs                     # 构建脚本
├── memory.x                     # 内存布局配置
├── rust-toolchain.toml          # Rust工具链配置
└── README.md                    # 本文件
```

## 测试分类

### 基础功能测试
- `hardware_test.rs` - 硬件基础功能测试
- `preempt_test.rs` - 基础抢占调度测试
- `bottom_test.rs` - 底层功能测试
- `sync_bottom_test.rs` - 同步底层测试

### 任务调度测试
- `switch_test.rs` - 异步任务切换
- `prio_test.rs` - 优先级测试
- `scheduling1_test.rs` - 调度算法测试1
- `scheduling2_test.rs` - 调度算法测试2

### 性能测试
- `time_performance.rs` - 时间性能测试
- `sync_time_performance.rs` - 同步时间性能测试
- `space_performance.rs` - 空间性能测试
- `stack_cost_test.rs` - 栈开销测试

### 外设通信测试
- `usart.rs` - 串口通信基础示例
- `usart_test.rs` - 串口通信测试
- `iic_test.rs` - I2C通信测试

### 综合测试
- `comprehensive_test.rs` - 综合功能测试

### 构建命令

```bash
# 基础构建
cargo build

# 运行特定测试
cargo run --bin hardware_test
cargo run --bin preempt_test
cargo run --bin comprehensive_test

# 启用日志功能构建
cargo build --features logs

# 发布版本构建
cargo build --release
```

### 环境变量配置

项目支持通过环境变量配置功能特性：

```bash
# 内存分区功能
export OS_MAX_MEM_PART=1

# 事件功能
export OS_Q_EN=1
export OS_MAX_QS=1
export OS_SEM_EN=1
export OS_MUTEX_EN=1

# 事件名称功能
export OS_EVENT_NAME_EN=1
```

## 配置说明

### Cargo.toml 特性

- **default = ["logs"]**: 默认启用日志功能
- **logs**: 包含所有日志功能（log-os, log-task, log-mem, log-timer, log-scheduler）
- **log-base**: 基础日志功能
- **log-os**: 操作系统日志
- **log-task**: 任务日志
- **log-mem**: 内存日志
- **log-timer**: 定时器日志
- **log-scheduler**: 调度器日志

### 构建优化

- **开发模式**: 代码生成单元为1，启用调试信息，优化级别为1
- **发布模式**: 代码生成单元为1，关闭调试信息，优化级别为3，启用LTO

## 依赖模块

目前依赖以下核心模块：

- **embassy-preempt-executor**: 执行器核心
- **embassy-preempt-app**: 应用层接口
- **embassy-preempt-log**: 日志功能
- **embassy-preempt-platform**: 平台抽象层