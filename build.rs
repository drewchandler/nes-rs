fn main() {
    // sdl2-sys 0.37 omits the registry library needed by its bundled SDL2.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
