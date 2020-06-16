fn main() {
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rerun-if-env-changed=SMP");
    println!("cargo:rerun-if-env-changed=BOARD");
    println!("cargo:rerun-if-env-changed=USER_IMG");

    let _arch: String = std::env::var("ARCH").unwrap();
    if let Ok(user_img) = std::env::var("USER_IMG") {
        println!("cargo:rerun-if-changed={}", user_img);
    }
}
