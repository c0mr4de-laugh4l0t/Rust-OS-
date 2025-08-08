fn main() {
    // linker script arg (if you're using one)
    println!("cargo:rustc-link-arg=-Tlinker.ld");

    // assemble context.S
    cc::Build::new()
        .file("src/context.S")
        .compile("context_switch");
}
