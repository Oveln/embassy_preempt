use std::env;
#[cfg(feature = "memory-x")]
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Check if STM32F401RE feature is enabled
    let has_stm32f401re = env::var("CARGO_FEATURE_STM32F401RE").is_ok();

    if has_stm32f401re {
        let chip_core_name = "stm32f401re";

        #[cfg(feature = "memory-x")]
        let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());

        #[cfg(feature = "memory-x")]
        println!(
            "cargo:rustc-link-search={}/src/memory_x/{}/",
            crate_dir.display(),
            chip_core_name
        );
    }
}