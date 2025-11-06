# Embassy Preempt Log

A logging crate for the Embassy Preempt RTOS that provides macros wrapping around `defmt` with optional logging levels based on feature flags.

## Features

- **log-base**: Base logging functionality with defmt
- **log-os**: OS-level logging
- **log-task**: Task-related logging
- **log-mem**: Memory management logging
- **log-timer**: Timer-related logging
- **log-scheduler**: Scheduler logging

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
embassy-preempt-log = { path = "../modules/embassy-preempt-log", features = ["log-os", "log-task"] }
```

Then import the macros:

```rust
use embassy_preempt_log::{debug, error, info, trace, warn};
```

## License

This project is licensed under the MIT OR Apache-2.0 license.