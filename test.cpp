#include <stdio.h>
#include <type_traits>

int main() {
    using T = __time_t;
    constexpr int a = sizeof(T);
    GETNCNT
    static_assert(std::is_same_v<T, long>);
    printf("xiaobenzhu\n");
    return 0;
}