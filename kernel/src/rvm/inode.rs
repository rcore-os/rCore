//! Implement INode for Rcore Virtual Machine

use alloc::collections::BTreeMap;
use core::any::Any;
use core::convert::{TryFrom, TryInto};
use core::mem::size_of;
use spin::RwLock;

use rcore_fs::vfs::*;
use rvm::{RvmError, RvmExitPacket, RvmResult, TrapKind, VcpuIo, VcpuReadWriteKind, VcpuState};

use super::into_fs_error;
use super::structs::{Guest, Vcpu};
use crate::syscall::{UserInOutPtr, UserInPtr, UserOutPtr};

const MAX_GUEST_NUM: usize = 64;
const MAX_VCPU_NUM: usize = 64;

const RVM_IO: u32 = 0xAE00;
const RVM_GUEST_CREATE: u32 = RVM_IO + 0x01;
const RVM_GUEST_ADD_MEMORY_REGION: u32 = RVM_IO + 0x02;
const RVM_GUEST_SET_TRAP: u32 = RVM_IO + 0x03;
const RVM_VCPU_CREATE: u32 = RVM_IO + 0x11;
const RVM_VCPU_RESUME: u32 = RVM_IO + 0x12;
const RVM_VCPU_READ_STATE: u32 = RVM_IO + 0x13;
const RVM_VCPU_WRITE_STATE: u32 = RVM_IO + 0x14;
const RVM_VCPU_INTERRUPT: u32 = RVM_IO + 0x15;

pub struct RvmINode {
    guests: RwLock<BTreeMap<usize, Guest>>,
    vcpus: RwLock<BTreeMap<usize, Vcpu>>,
}

#[repr(C)]
#[derive(Debug)]
struct RvmVcpuCreateArgs {
    vmid: u16,
    entry: u64,
}

#[repr(C)]
#[derive(Debug)]
struct RvmGuestAddMemoryRegionArgs {
    vmid: u16,
    guest_start_paddr: u64,
    memory_size: u64,
}

#[repr(C)]
#[derive(Debug)]
struct RvmGuestSetTrapArgs {
    vmid: u16,
    kind: u32,
    addr: u64,
    size: u64,
    key: u64,
}

#[repr(C)]
#[derive(Debug)]
struct RvmVcpuResumeArgs {
    vcpu_id: u16,
    packet: RvmExitPacket,
}

#[repr(C)]
#[derive(Debug)]
struct RvmVcpuStateArgs {
    vcpu_id: u16,
    kind: u32,
    user_buf_ptr: u64,
    buf_size: u64,
}

#[repr(C)]
#[derive(Debug)]
struct RvmVcpuInterruptArgs {
    vcpu_id: u16,
    vector: u32,
}

impl INode for RvmINode {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: false,
            write: false,
            error: false,
        })
    }
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 0,
            inode: 0,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::CharDevice,
            mode: 0o660,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(10, 232), // misc major, kvm minor
        })
    }
    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        match cmd {
            RVM_GUEST_CREATE => {
                info!("[RVM] ioctl RVM_GUEST_CREATE");
                self.guest_create().map_err(into_fs_error)
            }
            RVM_GUEST_ADD_MEMORY_REGION => {
                let args = UserInPtr::<RvmGuestAddMemoryRegionArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_GUEST_ADD_MEMORY_REGION {:x?}", args);
                self.guest_add_memory_region(
                    args.vmid as usize,
                    args.guest_start_paddr as usize,
                    args.memory_size as usize,
                )
                .map_err(into_fs_error)
            }
            RVM_GUEST_SET_TRAP => {
                let args = UserInPtr::<RvmGuestSetTrapArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_GUEST_SET_TRAP {:x?}", args);
                self.guest_set_trap(
                    args.vmid as usize,
                    args.kind.try_into().map_err(into_fs_error)?,
                    args.addr as usize,
                    args.size as usize,
                    args.key,
                )
                .map_err(into_fs_error)?;
                Ok(0)
            }
            RVM_VCPU_CREATE => {
                let args = UserInPtr::<RvmVcpuCreateArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_VCPU_CREATE {:x?}", args);
                self.vcpu_create(args.vmid as usize, args.entry)
                    .map_err(into_fs_error)
            }
            RVM_VCPU_RESUME => {
                let mut ptr = UserInOutPtr::<RvmVcpuResumeArgs>::from(data);
                let mut args = ptr.read().or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_VCPU_RESUME {:#x}", args.vcpu_id);
                args.packet = self
                    .vcpu_resume(args.vcpu_id as usize)
                    .map_err(into_fs_error)?;
                ptr.write(args).or(Err(FsError::DeviceError))?;
                Ok(0)
            }
            RVM_VCPU_READ_STATE => {
                let args = UserInPtr::<RvmVcpuStateArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_VCPU_READ_STATE {:#x?}", args);
                self.vcpu_read_state(
                    args.vcpu_id as usize,
                    args.kind,
                    args.user_buf_ptr as usize,
                    args.buf_size as usize,
                )
                .map_err(into_fs_error)?;
                Ok(0)
            }
            RVM_VCPU_WRITE_STATE => {
                let args = UserInPtr::<RvmVcpuStateArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_VCPU_WRITE_STATE {:#x?}", args);
                self.vcpu_write_state(
                    args.vcpu_id as usize,
                    args.kind,
                    args.user_buf_ptr as usize,
                    args.buf_size as usize,
                )
                .map_err(into_fs_error)?;
                Ok(0)
            }
            RVM_VCPU_INTERRUPT => {
                let args = UserInPtr::<RvmVcpuInterruptArgs>::from(data)
                    .read()
                    .or(Err(FsError::InvalidParam))?;
                info!("[RVM] ioctl RVM_VCPU_INTERRUPT {:#x?}", args);
                self.vcpu_interrupt(args.vcpu_id as usize, args.vector)
                    .map_err(into_fs_error)?;
                Ok(0)
            }
            _ => {
                warn!("[RVM] invalid ioctl number {:#x}", cmd);
                Err(FsError::InvalidParam)
            }
        }
    }
    fn mmap(&self, area: MMapArea) -> Result<()> {
        info!("[RVM] mmap {:x?}", area);
        Err(FsError::NotSupported)
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

impl RvmINode {
    pub fn new() -> Self {
        Self {
            guests: RwLock::new(BTreeMap::new()),
            vcpus: RwLock::new(BTreeMap::new()),
        }
    }

    fn get_free_vmid(&self) -> usize {
        (1..).find(|i| !self.guests.read().contains_key(i)).unwrap()
    }

    fn add_guest(&self, guest: Guest, vmid_option: Option<usize>) -> usize {
        let vmid = vmid_option.unwrap_or_else(|| self.get_free_vmid());
        self.guests.write().insert(vmid, guest);
        vmid
    }

    fn get_free_vcpu_id(&self) -> usize {
        (1..).find(|i| !self.vcpus.read().contains_key(i)).unwrap()
    }

    fn add_vcpu(&self, vcpu: Vcpu, vcpu_id_option: Option<usize>) -> usize {
        let vcpu_id = vcpu_id_option.unwrap_or_else(|| self.get_free_vcpu_id());
        self.vcpus.write().insert(vcpu_id, vcpu);
        vcpu_id
    }

    fn guest_create(&self) -> RvmResult<usize> {
        if rvm::check_hypervisor_feature() {
            let vmid = self.get_free_vmid();
            if vmid >= MAX_GUEST_NUM {
                warn!("[RVM] too many guests ({})", MAX_GUEST_NUM);
                return Err(RvmError::NoMemory);
            }
            self.add_guest(Guest::new()?, Some(vmid));
            Ok(vmid)
        } else {
            warn!("[RVM] no hardware support");
            Err(RvmError::NotSupported)
        }
    }

    fn guest_add_memory_region(&self, vmid: usize, gpaddr: usize, size: usize) -> RvmResult<usize> {
        if let Some(guest) = self.guests.read().get(&vmid) {
            Ok(guest.add_memory_region(gpaddr, size)?)
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn guest_set_trap(
        &self,
        vmid: usize,
        kind: TrapKind,
        addr: usize,
        size: usize,
        key: u64,
    ) -> RvmResult<()> {
        if let Some(guest) = self.guests.read().get(&vmid) {
            guest.inner.set_trap(kind, addr, size, None, key)
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn vcpu_create(&self, vmid: usize, entry: u64) -> RvmResult<usize> {
        if let Some(guest) = self.guests.read().get(&vmid) {
            let vcpu_id = self.get_free_vcpu_id();
            if vcpu_id >= MAX_VCPU_NUM {
                warn!("[RVM] too many vcpus ({})", MAX_VCPU_NUM);
                return Err(RvmError::NoMemory);
            }
            let vcpu = Vcpu::new(entry, guest.inner.clone())?;
            self.add_vcpu(vcpu, Some(vcpu_id));
            Ok(vcpu_id)
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn vcpu_resume(&self, vcpu_id: usize) -> RvmResult<RvmExitPacket> {
        if let Some(vcpu) = self.vcpus.write().get_mut(&vcpu_id) {
            Ok(vcpu.inner.lock().resume()?)
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn vcpu_read_state(
        &self,
        vcpu_id: usize,
        kind: u32,
        user_buf_ptr: usize,
        buf_size: usize,
    ) -> RvmResult<()> {
        if kind != VcpuReadWriteKind::VcpuState as u32 || buf_size != size_of::<VcpuState>() {
            return Err(RvmError::InvalidParam);
        }
        if let Some(vcpu) = self.vcpus.read().get(&vcpu_id) {
            let mut ptr = UserOutPtr::<VcpuState>::from(user_buf_ptr);
            let state = vcpu.inner.lock().read_state()?;
            ptr.write(state).or(Err(RvmError::InvalidParam))?;
            Ok(())
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn vcpu_write_state(
        &self,
        vcpu_id: usize,
        kind: u32,
        user_buf_ptr: usize,
        buf_size: usize,
    ) -> RvmResult<()> {
        if let Some(vcpu) = self.vcpus.write().get_mut(&vcpu_id) {
            match VcpuReadWriteKind::try_from(kind) {
                Ok(VcpuReadWriteKind::VcpuState) => {
                    if buf_size != size_of::<VcpuState>() {
                        return Err(RvmError::InvalidParam);
                    }
                    let ptr = UserInPtr::<VcpuState>::from(user_buf_ptr);
                    let state = ptr.read().or(Err(RvmError::InvalidParam))?;
                    vcpu.inner.lock().write_state(&state)
                }
                Ok(VcpuReadWriteKind::VcpuIo) => {
                    if buf_size != size_of::<VcpuIo>() {
                        return Err(RvmError::InvalidParam);
                    }
                    let ptr = UserInPtr::<VcpuIo>::from(user_buf_ptr);
                    let state = ptr.read().or(Err(RvmError::InvalidParam))?;
                    vcpu.inner.lock().write_io_state(&state)
                }
                Err(_) => return Err(RvmError::InvalidParam),
            }
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    fn vcpu_interrupt(&self, vcpu_id: usize, vector: u32) -> RvmResult<()> {
        if let Some(vcpu) = self.vcpus.write().get_mut(&vcpu_id) {
            vcpu.inner.lock().virtual_interrupt(vector)
        } else {
            Err(RvmError::InvalidParam)
        }
    }

    // TODO: remove guest & vcpu
}
