rCore Kernel Module Template: Rust
This may be a good startpoint for developing your own kernel module. Just copy the folder and start your work.
Known problems:
- You have to execute the given build script.
- The kernel and the module have to follow the same toolchain strictly. This means you are likely to rebuild the module after you build the kernel with the same tool.
  This makes developing "portable" kernel module a severe problem, although not rebuilding the module rarely cause problems.
