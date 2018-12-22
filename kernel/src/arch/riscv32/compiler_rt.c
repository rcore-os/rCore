// http://llvm.org/docs/Atomics.html#libcalls-atomic

typedef unsigned u32;

// K210 doesn't support atomic operation on 0x40000000 (io port)
// We have to detect it and move it to 0x80000000
inline u32* fix_ptr32(u32 *src) {
    return (u32)src < 0x80000000?
            (u32*)((u32)src + 0x40000000):
            src;
}

u32 __atomic_load_1(u32 *src) {
    src = fix_ptr32(src);
    u32 res = 0;
    __asm__ __volatile__("amoadd.w %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res;
}

u32 __atomic_load_2(u32 *src) {
    src = fix_ptr32(src);
    u32 res = 0;
    __asm__ __volatile__("amoadd.w %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res;
}

u32 __atomic_load_4(u32 *src) {
    src = fix_ptr32(src);
    u32 res = 0;
    __asm__ __volatile__("amoadd.w %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res;
}

void __atomic_store_4(u32 *dst, u32 val) {
    dst = fix_ptr32(dst);
    __asm__ __volatile__("amoswap.w zero, %0, (%1)" :: "r"(val), "r"(dst) : "memory");
}

char __atomic_compare_exchange_4(u32* dst, u32* expected, u32 desired) {
    dst = fix_ptr32(dst);
    u32 val, expect, result;
    // val = *dst
    __asm__ __volatile__("lw %0, (%1)" : "=r"(expect) : "r" (expected) : "memory");
    __asm__ __volatile__("lr.w %0, (%1)" : "=r"(val) : "r"(dst) : "memory");

    // if (val != *expected) goto fail;
    if (val != expect) goto __atomic_compare_exchange_4_fail;

    // Try: *dst = desired. If success, result = 0, otherwise result != 0.
    __asm__ __volatile__("sc.w %0, %1, (%2)" : "=r"(result) : "r"(desired), "r"(dst) : "memory");
    return result == 0;

    __atomic_compare_exchange_4_fail:

    // *expected should always equal to the previous value of *dst
    *expected = val;
    return 0;
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
inline u64* fix_ptr64(u64 *src) {
    return (u64)src < 0x80000000?
            (u64*)((u64)src + 0x40000000):
            src;
}

u64 __atomic_load_8(u64 *src) {
    src = fix_ptr64(src);
    u64 res = 0;
    __asm__ __volatile__("amoadd.d %0, zero, (%1)" : "=r"(res) : "r"(src) : "memory");
    return res;
}

void __atomic_store_8(u64 *dst, u64 val) {
    dst = fix_ptr64(dst);
    __asm__ __volatile__("amoswap.d zero, %0, (%1)" :: "r"(val), "r"(dst) : "memory");
}

char __atomic_compare_exchange_8(u64* dst, u64* expected, u64 desired) {
    dst = fix_ptr64(dst);
    u64 val, expect, result;
    // val = *dst
    __asm__ __volatile__("ld %0, (%1)" : "=r"(expect) : "r" (expected) : "memory");
    __asm__ __volatile__("lr.d %0, (%1)" : "=r"(val) : "r"(dst) : "memory");

    // if (val != *expected) goto fail;
    if (val != expect) goto __atomic_compare_exchange_8_fail;

    // Try: *dst = desired. If success, result = 0, otherwise result != 0.
    __asm__ __volatile__("sc.d %0, %1, (%2)" : "=r"(result) : "r"(desired), "r"(dst) : "memory");
    return result == 0;

    __atomic_compare_exchange_8_fail:

    // *expected should always equal to the previous value of *dst
    *expected = val;
    return 0;
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
