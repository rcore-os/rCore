extern crate cc;
use std::process::Command;

fn main() {
	let output = Command::new("uname").output()
						 .expect("failed to get uname");
	let compiler = match output.stdout.as_slice() {
		b"Darwin\n" => "x86_64-elf-gcc",
		b"Linux\n" => "gcc",
		_ => panic!("unknown os")
	};
    cc::Build::new()
		.compiler(compiler)
		.file("src/arch/x86_64/driver/apic/lapic.c")
		.compile("cobj");
}