use std::env;
fn is_any_log_feature_enabled() -> bool {
    const LOG_FEATURE_ENV_VARS: &[&str] = &[
        "CARGO_FEATURE_LOG_OS",
        "CARGO_FEATURE_LOG_TASK",
        "CARGO_FEATURE_LOG_MEM",
        "CARGO_FEATURE_LOG_TIMER",
        "CARGO_FEATURE_LOG_SCHEDULER",
    ];

    LOG_FEATURE_ENV_VARS.iter().any(|&var| env::var(var).is_ok())
}
fn main() {
    println!("cargo::rustc-check-cfg=cfg(log_enabled)");
    if is_any_log_feature_enabled() {
        println!("cargo:rustc-cfg=log_enabled");
        // 编译选项的可选："-C", "link-arg=-Tdefmt.x", 开了logs的时候才会加入
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }
}
