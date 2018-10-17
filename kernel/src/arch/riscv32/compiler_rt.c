

// fn __atomic_load_1_workaround(src: *const u8) -> u8;
// fn __atomic_load_2_workaround(src: *const u16) -> u16;
// fn __atomic_load_4_workaround(src: *const u32) -> u32;
// fn __atomic_store_1_workaround(dst: *mut u8, val: u8);
// fn __atomic_store_4_workaround(dst: *mut u32, val: u32);
// fn __atomic_compare_exchange_1_workaround(dst: *mut u8, expected: *mut u8, desired: u8) -> bool;
// fn __atomic_compare_exchange_4_workaround(dst: *mut u32, expected: *mut u32, desired: u32) -> bool;

char __atomic_load_1_workaround(char *src) {
    char res = 0;
    __asm__ __volatile__("amoadd.w.rl %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res; 
}

short __atomic_load_2_workaround(short *src) {
    short res = 0;
    __asm__ __volatile__("amoadd.w.rl %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res; 
}

int __atomic_load_4_workaround(int *src) {
    int res = 0;
    __asm__ __volatile__("amoadd.w.rl %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res; 
}

char __atomic_store_1_workaround(char *dst, char val) {
    __asm__ __volatile__("amoswap.w.aq zero, %0, (%1)" :: "r"(val), "r"(dst) : "memory");
}

int __atomic_store_4_workaround(int *dst, int val) {
    __asm__ __volatile__("amoswap.w.aq zero, %0, (%1)" :: "r"(val), "r"(dst) : "memory");
}

char __atomic_compare_exchange_1_workaround(char* dst, char* expected, char desired) {
    char val = 0;
    __asm__ __volatile__("lr.w %0, (%1)" : "=r"(val) : "r"(dst) : "memory");
    if (val == *expected) {
        int sc_ret = 0;
        __asm__ __volatile__("sc.w %0, %1, (%2)" : "=r"(sc_ret) : "r"(desired), "r"(dst) : "memory");
        return sc_ret == 0;
    }
    return 0;
}

char __atomic_compare_exchange_4_workaround(int* dst, int* expected, int desired) {
    int val = 0;
    __asm__ __volatile__("lr.w %0, (%1)" : "=r"(val) : "r"(dst) : "memory");
    if (val == *expected) {
        int sc_ret = 0;
        __asm__ __volatile__("sc.w %0, %1, (%2)" : "=r"(sc_ret) : "r"(desired), "r"(dst) : "memory");
        return sc_ret == 0;
    }
    return 0;
}