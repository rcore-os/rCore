fn main() {
    let path = "../../kernel/target/x86_64/release/deps";
    println!("cargo:rustc-link-search=all={}", path);
    let path = "../../kernel/target/aarch64/release/deps";
    println!("cargo:rustc-link-search=all={}", path);
}
