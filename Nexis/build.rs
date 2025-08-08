// build.rs
fn main() {
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    // instruct cargo to assemble context.S
    cc::Build::new()
        .file("src/context.S")
        .compile("context_switch");
}
