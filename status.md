## ucore_os_lab port status
#### lab1: 

- [x] Basic init：LocalAPIC，IOAPIC，GDT，PIC
- [x] Device：Keyboard，Serial，PIT，IDE
- [x] Interrupt & Trapframe
- [x] ※ Muilt-core startup

#### lab2: Physical memory management

- [x] Frame allocator：Naive
- [ ] Frame allocator：First Fit，Best Fit，Worst Fit，Buddy，Slab
- [x] Higher half kernel space
- [x] Kernel remap

#### lab3: Virtual memory management

- [x] Page table
- [x] Heap allocator：LinkedList (Rust crate)
- [x] ※ Stack allocator：Naive
- [x] MM & VMA
- [x] Copy on write
- [ ] Swap

#### lab4: Kernel thread

- [x] idleproc
- [x] initproc
- [x] fork
- [ ] Scheduler thread

#### lab5: User thread

- [x] Run xv6 64bit user programs：See the list below
- [x] Run ucore 32bit user programs：See the list below

#### lab6: Schedule

- [x] Schedule framework
- [x] RRScheduler
- [x] StrideScheduler

#### lab7: Synchronization 

- [x] Mutex：Rust core lib built-in
- [ ] Semaphore
- [ ] Monitor
- [ ] Dinning Philosophers Problem

#### lab8: File system

- [x] Simple file system
- [x] Load user programs from .img
- [ ] FS framework for process
- [ ] Device IO


## uCore 32bit user programs pass status
- [ ] badarg
- [ ] badsegment
- [x] divzero
- [x] exit
- [x] faultread
- [x] faultreadkernel
- [x] forktest
- [x] forktree
- [x] hello
- [ ] ls
- [x] matrix
- [ ] pgdir
- [x] priority
- [ ] sh
- [x] sleep
- [x] sleepkill
- [x] softint
- [x] spin
- [x] testbss
- [x] waitkill
- [x] yield

## xv6 64bit user programs pass status
- [ ] cat
- [ ] chmod
- [ ] echo
- [ ] forktest
- [ ] grep
- [ ] init
- [ ] kill
- [ ] ln
- [ ] ls
- [ ] mkdir
- [ ] rm
- [ ] sh
- [ ] stressfs
- [ ] usertests
- [ ] wc
- [ ] zombie
