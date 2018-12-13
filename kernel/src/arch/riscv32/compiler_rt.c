// http://llvm.org/docs/Atomics.html#libcalls-atomic

int __atomic_load_4(int *src) {
    int res = 0;
    __asm__ __volatile__("amoadd.w.rl %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res;
}

int __atomic_store_4(int *dst, int val) {
    __asm__ __volatile__("amoswap.w.aq zero, %0, (%1)" :: "r"(val), "r"(dst) : "memory");
}

char __atomic_compare_exchange_4(int* dst, int* expected, int desired) {
    int val;
    // val = *dst
    __asm__ __volatile__("lr.w %0, (%1)" : "=r"(val) : "r"(dst) : "memory");
    if (val == *expected) {
        int result;
        // Try: *dst = desired. If success, result = 0, otherwise result != 0.
        __asm__ __volatile__("sc.w %0, %1, (%2)" : "=r"(result) : "r"(desired), "r"(dst) : "memory");
        return result == 0;
    }
    // *expected should always equal to the previous value of *dst
    *expected = val;
    return 0;
}

int __atomic_fetch_add_4(int* ptr, int val) {
    int res;
    __asm__ __volatile__("amoadd.w.rl %0, %1, (%2)" : "=r"(res) : "r"(val), "r"(ptr) : "memory");
    return res;
}

int __atomic_fetch_sub_4(int* ptr, int val) {
    int res;
    __asm__ __volatile__("amoadd.w.rl %0, %1, (%2)" : "=r"(res) : "r"(-val), "r"(ptr) : "memory");
    return res;
}