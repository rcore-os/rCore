Tools that are used to fill in kernel symbols into rcore ELF file.
The tool will use `nm` to extract symbols from the kernel (a bit like System.map), and put it back into the `rcore_symbol_table` section.
To reduce the size required, the symbol table will be compressed using gzip.
The tool tries to limit its dependencies. Only necessary tools (bash, objdump, nm, gzip, grep, dd, python3) are required to run the script.
TODO: Why don't we just do the job using a single Python script?
