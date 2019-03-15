pub fn translate(id: usize) -> usize {
    match id {
        17 => 79, // getcwd
        25 => 72, // fcntl
        29 => 16, // ioctl
        49 => 80, // chdir
        50 => 81, // fchdir
        51 => 161, // chroot
        59 => 293, // pipe2
        61 => 217, // getdents64
        62 => 8, // lseek
        63 => 0, // read
        64 => 1, // write
        65 => 19, // readv
        66 => 20, // writev
        67 => 17, // pread64
        68 => 18, // pwrite64
        69 => 295, // preadv
        70 => 296, // pwritev
        124 => 24, // sched_yield
        166 => 95, // umask
        172 => 39, // getpid
        173 => 110, // getppid
        174 => 102, // getuid
        175 => 107, // geteuid
        176 => 104, // getgid
        177 => 108, // getegid
        214 => 12, // brk
        220 => 56, // clone
        221 => 59, // execve
        260 => 61, // wait4
        _ => panic!("riscv syscall id {} not found", id)
    }
}
