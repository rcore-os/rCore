use super::*;

pub fn sys_arch_prctl(code: i32, addr: usize, tf: &mut TrapFrame) -> SysResult {
    const ARCH_SET_FS: i32 = 0x1002;
    match code {
        #[cfg(target_arch = "x86_64")]
        ARCH_SET_FS => {
            info!("sys_arch_prctl: set FS to {:#x}", addr);
            tf.fsbase = addr;
            Ok(0)
        }
        _ => Err(SysError::Inval),
    }
}
