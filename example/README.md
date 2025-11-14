# Embassy Preempt Example 使用说明

这个示例项目展示了如何使用 Embassy Preempt 实时操作系统进行嵌入式开发。

## 项目结构

```
example/
├── src/bin/                    # 二进制示例程序
├── tests/                      # 单元测试
└── Cargo.toml
```

## 快速开始

### 1. 编译项目

```bash
# 编译所有二进制文件
cargo build

# 编译特定示例
cargo build --bin space_performance
cargo build --bin usart
```

### 2. 运行测试

```bash
# 运行所有单元测试
cargo test

# 运行特定测试
cargo test --test prio_test
cargo test --test task_create_test
```

### 3. 运行示例

注意：这些是为嵌入式平台设计的程序，需要相应的硬件环境。

```bash
# 编译并运行（需要硬件支持）
cargo run --bin space_performance
cargo run --bin usart
```

## 主要功能模块

### 性能测试
- `space_performance.rs` - 内存空间使用性能测试
- `time_performance.rs` - 执行时间性能测试
- `stack_cost_test.rs` - 栈空间成本分析
- `sync_time_performance.rs` - 同步操作时间性能

### 调度测试
- `prio_test.rs` - 优先级调度算法验证
- `scheduling1_test.rs` - 基础调度测试
- `scheduling2_test.rs` - 调度功能测试
- `preempt_test.rs` - 任务抢占机制测试

### 硬件接口
- `usart.rs` / `usart_test.rs` - 串口通信
- `iic_test.rs` - I2C总线通信
- `hardware_test.rs` - 硬件外设测试

### 系统功能
- `task_create_test.rs` - 任务创建和管理
- `comprehensive_test.rs` - 综合功能测试
- `bottom_test.rs` - 底层系统调用测试

## 配置说明

### Cargo.toml 关键配置

项目使用了以下关键配置来解决编译问题：

1. **二进制文件测试禁用** - 为每个二进制文件添加了 `test = false` 配置
2. **测试框架配置** - 使用 `defmt-test` 进行嵌入式测试
3. **特性标志** - 支持日志记录、调试等功能

```toml
# 禁用二进制文件的测试编译
[[bin]]
name = "space_performance"
test = false

# 配置嵌入式测试
[[test]]
name = "prio_test"
harness = false
```