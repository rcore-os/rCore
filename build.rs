extern crate cc;

fn main() {
	let mut build = cc::Build::new();

	let compiler = if build.get_compiler().is_like_clang() 
					{ "x86_64-elf-gcc" } else {"gcc"};
    build.compiler(compiler)
         .file("src/test.c")
         .compile("cobj");
}