// http://llvm.org/docs/Atomics.html#libcalls-atomic

inline void mb() {
    __asm__ __volatile__("fence" ::: "memory");
}

typedef unsigned u32;

// K210 doesn't support atomic operation on 0x40000000 (io port)
// We have to detect it and move it to 0x80000000
inline u32* fix_ptr32(u32 *ptr) {
    return ptr < (u32*)0x80000000?
            ptr + 0x40000000 / sizeof(u32):
            ptr;
}

u32 __atomic_load_1(u32 *ptr) {
    ptr = fix_ptr32(ptr);
    return *ptr;
}

u32 __atomic_load_2(u32 *ptr) {
    ptr = fix_ptr32(ptr);
    return *ptr;
}

// relaxed
u32 __atomic_load_4(u32 *ptr) {
    ptr = fix_ptr32(ptr);
    return *ptr;
}

// release
void __atomic_store_4(u32 *ptr, u32 val) {
    ptr = fix_ptr32(ptr);
    mb();
    __asm__ __volatile__("amoswap.w zero, %0, (%1)" :: "r"(val), "r"(ptr) : "memory");
}

// strong, acquire
char __atomic_compare_exchange_4(u32* ptr, u32* expected, u32 desired) {
    ptr = fix_ptr32(ptr);
    u32 val, expect = *expected, result, ret;
    while(1) {
        __asm__ __volatile__("lr.w.aq %0, (%1)" : "=r"(val) : "r"(ptr) : "memory");

        ret = val == expect;
        if(!ret) {
            // *expected should always equal to the previous value of *ptr
            *expected = val;
            return ret;
        }

        // Try: *ptr = desired. If success, result == 0, otherwise result != 0.
        __asm__ __volatile__("sc.w.aq %0, %1, (%2)" : "=r"(result) : "r"(desired), "r"(ptr) : "memory");
        if(result == 0) {
            return ret;
        }
    }
}

u32 __atomic_fetch_add_4(u32* ptr, u32 val) {
    ptr = fix_ptr32(ptr);
    u32 res;
    __asm__ __volatile__("amoadd.w %0, %1, (%2)" : "=r"(res) : "r"(val), "r"(ptr) : "memory");
    return res;
}

u32 __atomic_fetch_sub_4(u32* ptr, u32 val) {
    ptr = fix_ptr32(ptr);
    u32 res;
    __asm__ __volatile__("amoadd.w %0, %1, (%2)" : "=r"(res) : "r"(-val), "r"(ptr) : "memory");
    return res;
}

#if __riscv_xlen == 64
typedef unsigned long long u64;

// K210 doesn't support atomic operation on 0x40000000 (io port)
// We have to detect it and move it to 0x80000000
inline u64* fix_ptr64(u64 *ptr) {
    return ptr < (u64*)0x80000000?
            ptr + 0x40000000 / sizeof(u64):
            ptr;
}

// relaxed
u64 __atomic_load_8(u64 *ptr) {
    ptr = fix_ptr64(ptr);
    return *ptr;
}

// release
void __atomic_store_8(u64 *ptr, u64 val) {
    ptr = fix_ptr64(ptr);
    mb();
    __asm__ __volatile__("amoswap.d zero, %0, (%1)" :: "r"(val), "r"(ptr) : "memory");
}

// strong, acquire
char __atomic_compare_exchange_8(u64* ptr, u64* expected, u64 desired) {
    ptr = fix_ptr64(ptr);
    u64 val, expect = *expected, result, ret;
    while(1) {
        __asm__ __volatile__("lr.d.aq %0, (%1)" : "=r"(val) : "r"(ptr) : "memory");

        ret = val == expect;
        if(!ret) {
            // *expected should always equal to the previous value of *ptr
            *expected = val;
            return ret;
        }

        // Try: *ptr = desired. If success, result == 0, otherwise result != 0.
        __asm__ __volatile__("sc.d.aq %0, %1, (%2)" : "=r"(result) : "r"(desired), "r"(ptr) : "memory");
        if(result == 0) {
            return ret;
        }
    }
}

u64 __atomic_fetch_add_8(u64* ptr, u64 val) {
    ptr = fix_ptr64(ptr);
    u64 res;
    __asm__ __volatile__("amoadd.d %0, %1, (%2)" : "=r"(res) : "r"(val), "r"(ptr) : "memory");
    return res;
}

u64 __atomic_fetch_sub_8(u64* ptr, u64 val) {
    ptr = fix_ptr64(ptr);
    u64 res;
    __asm__ __volatile__("amoadd.d %0, %1, (%2)" : "=r"(res) : "r"(-val), "r"(ptr) : "memory");
    return res;
}
#endif
