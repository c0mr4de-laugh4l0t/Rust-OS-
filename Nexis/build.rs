// build.rs — instruct rustc to use our linker script
fn main() {
    // Tell rustc to pass the linker script to the linker
    println!("cargo:rustc-link-arg=-Tlinker.ld");
}
