fn main() {
    // The platform sub-crates (jh7110, stm32f4, ch32v3) emit their own
    // cargo:rustc-link-search directives in their build.rs files.
    // This parent crate doesn't need to know anything about paths.
    println!("cargo:rerun-if-changed=build.rs");
}
