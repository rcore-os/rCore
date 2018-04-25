//! Copy from Redox

#[allow(dead_code)]
#[repr(packed)]
pub struct ScratchRegisters {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

impl ScratchRegisters {
    pub fn dump(&self) {
        println!("RAX:   {:>016X}", { self.rax });
        println!("RCX:   {:>016X}", { self.rcx });
        println!("RDX:   {:>016X}", { self.rdx });
        println!("RDI:   {:>016X}", { self.rdi });
        println!("RSI:   {:>016X}", { self.rsi });
        println!("R8:    {:>016X}", { self.r8 });
        println!("R9:    {:>016X}", { self.r9 });
        println!("R10:   {:>016X}", { self.r10 });
        println!("R11:   {:>016X}", { self.r11 });
    }
}

macro_rules! scratch_push {
    () => (asm!(
        "push rax
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11"
        : : : : "intel", "volatile"
    ));
}

macro_rules! scratch_pop {
    () => (asm!(
        "pop r11
        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rdx
        pop rcx
        pop rax"
        : : : : "intel", "volatile"
    ));
}

#[allow(dead_code)]
#[repr(packed)]
pub struct PreservedRegisters {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,
}

impl PreservedRegisters {
    pub fn dump(&self) {
        println!("RBX:   {:>016X}", { self.rbx });
        println!("RBP:   {:>016X}", { self.rbp });
        println!("R12:   {:>016X}", { self.r12 });
        println!("R13:   {:>016X}", { self.r13 });
        println!("R14:   {:>016X}", { self.r14 });
        println!("R15:   {:>016X}", { self.r15 });
    }
}

macro_rules! preserved_push {
    () => (asm!(
        "push rbx
        push rbp
        push r12
        push r13
        push r14
        push r15"
        : : : : "intel", "volatile"
    ));
}

macro_rules! preserved_pop {
    () => (asm!(
        "pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx"
        : : : : "intel", "volatile"
    ));
}

macro_rules! fs_push {
    () => (asm!(
        "push fs
        mov rax, 0x18
        mov fs, ax"
        : : : : "intel", "volatile"
    ));
}

macro_rules! fs_pop {
    () => (asm!(
        "pop fs"
        : : : : "intel", "volatile"
    ));
}

#[allow(dead_code)]
#[repr(packed)]
pub struct IretRegisters {
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
}

impl IretRegisters {
    pub fn dump(&self) {
        println!("RFLAG: {:>016X}", { self.rflags });
        println!("CS:    {:>016X}", { self.cs });
        println!("RIP:   {:>016X}", { self.rip });
    }
}

macro_rules! iret {
    () => (asm!(
        "iretq"
        : : : : "intel", "volatile"
    ));
}

/// Create an interrupt function that can safely run rust code
#[macro_export]
macro_rules! interrupt {
    ($name:ident, $func:block) => {
        #[naked]
        pub unsafe extern fn $name () {
            #[inline(never)]
            unsafe fn inner() {
                $func
            }

            // Push scratch registers
            scratch_push!();
            fs_push!();

            // Map kernel
//            $crate::arch::x86_64::pti::map();

            // Call inner rust function
            inner();

            // Unmap kernel
//            $crate::arch::x86_64::pti::unmap();

            // Pop scratch registers and return
            fs_pop!();
            scratch_pop!();
            iret!();
        }
    };
}

#[allow(dead_code)]
#[repr(packed)]
pub struct InterruptStack {
    pub fs: usize,
    pub scratch: ScratchRegisters,
    pub iret: IretRegisters,
}

impl InterruptStack {
    pub fn dump(&self) {
        self.iret.dump();
        self.scratch.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
}

#[macro_export]
macro_rules! interrupt_stack {
    ($name:ident, $stack: ident, $func:block) => {
        #[naked]
        pub unsafe extern fn $name () {
            #[inline(never)]
            unsafe fn inner($stack: &mut InterruptStack) {
                $func
            }

            // Push scratch registers
            scratch_push!();
            fs_push!();

            // Get reference to stack variables
            let rsp: usize;
            asm!("" : "={rsp}"(rsp) : : : "intel", "volatile");

            // Map kernel
//            $crate::arch::x86_64::pti::map();

            // Call inner rust function
            inner(&mut *(rsp as *mut InterruptStack));

            // Unmap kernel
//            $crate::arch::x86_64::pti::unmap();

            // Pop scratch registers and return
            fs_pop!();
            scratch_pop!();
            iret!();
        }
    };
}

#[allow(dead_code)]
#[repr(packed)]
pub struct InterruptErrorStack {
    pub fs: usize,
    pub scratch: ScratchRegisters,
    pub code: usize,
    pub iret: IretRegisters,
}

impl InterruptErrorStack {
    pub fn dump(&self) {
        self.iret.dump();
        println!("CODE:  {:>016X}", { self.code });
        self.scratch.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
}

#[macro_export]
macro_rules! interrupt_error {
    ($name:ident, $stack:ident, $func:block) => {
        #[naked]
        pub unsafe extern fn $name () {
            #[inline(never)]
            unsafe fn inner($stack: &InterruptErrorStack) {
                $func
            }

            // Push scratch registers
            scratch_push!();
            fs_push!();

            // Get reference to stack variables
            let rsp: usize;
            asm!("" : "={rsp}"(rsp) : : : "intel", "volatile");

            // Map kernel
//            $crate::arch::x86_64::pti::map();

            // Call inner rust function
            inner(&*(rsp as *const InterruptErrorStack));

            // Unmap kernel
//            $crate::arch::x86_64::pti::unmap();

            // Pop scratch registers, error code, and return
            fs_pop!();
            scratch_pop!();
            asm!("add rsp, 8" : : : : "intel", "volatile");
            iret!();
        }
    };
}

#[allow(dead_code)]
#[repr(packed)]
pub struct InterruptStackP {
    pub fs: usize,
    pub preserved: PreservedRegisters,
    pub scratch: ScratchRegisters,
    pub iret: IretRegisters,
}

impl InterruptStackP {
    pub fn dump(&self) {
        self.iret.dump();
        self.scratch.dump();
        self.preserved.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
}

#[macro_export]
macro_rules! interrupt_stack_p {
    ($name:ident, $stack: ident, $func:block) => {
        #[naked]
        pub unsafe extern fn $name () {
            #[inline(never)]
            unsafe fn inner($stack: &mut InterruptStackP) {
                $func
            }

            // Push scratch registers
            scratch_push!();
            preserved_push!();
            fs_push!();

            // Get reference to stack variables
            let rsp: usize;
            asm!("" : "={rsp}"(rsp) : : : "intel", "volatile");

            // Map kernel
//            $crate::arch::x86_64::pti::map();

            // Call inner rust function
            inner(&mut *(rsp as *mut InterruptStackP));

            // Unmap kernel
//            $crate::arch::x86_64::pti::unmap();

            // Pop scratch registers and return
            fs_pop!();
            preserved_pop!();
            scratch_pop!();
            iret!();
        }
    };
}

#[allow(dead_code)]
#[repr(packed)]
pub struct InterruptErrorStackP {
    pub fs: usize,
    pub preserved: PreservedRegisters,
    pub scratch: ScratchRegisters,
    pub code: usize,
    pub iret: IretRegisters,
}

impl InterruptErrorStackP {
    pub fn dump(&self) {
        self.iret.dump();
        println!("CODE:  {:>016X}", { self.code });
        self.scratch.dump();
        self.preserved.dump();
        println!("FS:    {:>016X}", { self.fs });
    }
}

#[macro_export]
macro_rules! interrupt_error_p {
    ($name:ident, $stack:ident, $func:block) => {
        #[naked]
        pub unsafe extern fn $name () {
            #[inline(never)]
            unsafe fn inner($stack: &InterruptErrorStackP) {
                $func
            }

            // Push scratch registers
            scratch_push!();
            preserved_push!();
            fs_push!();

            // Get reference to stack variables
            let rsp: usize;
            asm!("" : "={rsp}"(rsp) : : : "intel", "volatile");

            // Map kernel
//            $crate::arch::x86_64::pti::map();

            // Call inner rust function
            inner(&*(rsp as *const InterruptErrorStackP));

            // Unmap kernel
//            $crate::arch::x86_64::pti::unmap();

            // Pop scratch registers, error code, and return
            fs_pop!();
            preserved_pop!();
            scratch_pop!();
            asm!("add rsp, 8" : : : : "intel", "volatile");
            iret!();
        }
    };
}
