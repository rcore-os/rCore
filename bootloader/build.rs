fn main() {
    println!("cargo:rerun-if-env-changed=PAYLOAD");
    println!("cargo:rerun-if-env-changed=DTB");

    if let Ok(payload) = std::env::var("PAYLOAD") {
        println!("cargo:rerun-if-changed={}", payload);
    }
    if let Ok(dtb) = std::env::var("DTB") {
        println!("cargo:rerun-if-changed={}", dtb);
    }
}
