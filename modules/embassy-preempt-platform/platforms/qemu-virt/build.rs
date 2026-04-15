use std::env;

fn main() {
    // Only emit link-search if memory-x feature is enabled in the parent
    // The parent crate will have the feature, and we can detect if we're being built
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Emit the linker search path for memory.x
    // This will be picked up by any crate that depends on jh7110
    println!("cargo:rustc-link-search={}", manifest_dir);

    println!("cargo:rerun-if-changed=memory.x");
}
