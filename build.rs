extern crate cc;

fn main() {
    cc::Build::new()
		.file("src/arch/x86_64/driver/apic/lapic.c")
		.file("src/arch/x86_64/driver/keyboard/keyboard.c")
		.compile("cobj");
}