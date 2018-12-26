#ifdef RISCV_QEMU
# define SYS_write 64
# define SYS_exit 93
long syscall(long a0, long a1, long a2, long a3, long a4, long a5, long a6) {
	register long _0 __asm__("a7") = a0;
	register long _1 __asm__("a0") = a1;
	register long _2 __asm__("a1") = a2;
	register long _3 __asm__("a2") = a3;
	register long _4 __asm__("a3") = a4;
	register long _5 __asm__("a4") = a5;
	register long _6 __asm__("a5") = a6;
	__asm__ __volatile__("ecall":::);
	return _1;
}

#else
# define SYS_write 103
# define SYS_exit 1
# define SYS_fork 2
# define SYS_putc 30
# define SYS_getpid 18
# define SYS_sleep 11

long syscall(long a0, long a1, long a2, long a3, long a4, long a5, long a6) {
	register long _0 __asm__("x10") = a0;
	register long _1 __asm__("x11") = a1;
	register long _2 __asm__("x12") = a2;
	register long _3 __asm__("x13") = a3;
	register long _4 __asm__("x14") = a4;
	register long _5 __asm__("x15") = a5;
	register long _6 __asm__("x16") = a6;
	__asm__ __volatile__("ecall":::);
	return _0;
}
#endif

const char* welcome_msg = "hello world!\n";
const char* hexch = "0123456789ABCDEF";

void putc(char c) {
	syscall(SYS_putc, c, 0, 0, 0, 0, 0);
}

void putstr(const char* s) {
	for (; *s; s++)
		syscall(SYS_putc, *s, 0, 0, 0, 0, 0);
}

void putint_hex(long v) {
	char ch[18];
	ch[16] = 'H';
	ch[17] = 0;
	for (int i = 15; i >= 0; i--) {
		ch[i] = hexch[v & 15];
		v >>= 4;
	}
	putstr(ch);
}

void _start() {
	putstr(welcome_msg);
	putc('\n');

	putstr("my pid is ");
	long v = syscall(SYS_getpid, 0, 0, 0, 0, 0, 0);
	putint_hex(v);
	putc('\n');

	long v1 = syscall(SYS_fork, 0, 0, 0, 0, 0, 0);
	putstr("fork returned: ");
	putint_hex(v1);
	putc('\n');
	if (v1 != 0) {
		putstr("parent sleeping");
		putc('\n');
		syscall(SYS_sleep, 200, 0, 0, 0, 0, 0);
	}
	putstr("my pid is ");
	v = syscall(SYS_getpid, 0, 0, 0, 0, 0, 0);
	putint_hex(v);
	putc('\n');

	putint_hex(v);
	putstr(" is exiting");
	putc('\n');
	syscall(SYS_exit, 0, 0, 0, 0, 0, 0);
}
