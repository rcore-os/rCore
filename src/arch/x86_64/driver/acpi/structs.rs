// Reference: xv6-x86_64 acpi.h
// Copy from crate 'acpica-sys'

use util::{Checkable, bytes_sum};
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct rsdp {
    pub Signature: [u8; 8],
    pub Checksum: u8,
    pub OemId: [i8; 6],
    pub Revision: u8,
    pub RsdtPhysicalAddress: u32,
    pub Length: u32,
    pub XsdtPhysicalAddress: u64,
    pub ExtendedChecksum: u8,
    pub Reserved: [u8; 3],
}

impl Checkable for rsdp {
    fn check(&self) -> bool {
        &self.Signature == b"RSD PTR " && bytes_sum(self) == 0
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct header {
    pub Signature: [u8; 4],
    pub Length: u32,
    pub Revision: u8,
    pub Checksum: u8,
    pub OemId: [i8; 6],
    pub OemTableId: [i8; 8],
    pub OemRevision: u32,
    pub AslCompilerId: [i8; 4],
    pub AslCompilerRevision: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct rsdt {
    pub Header: header,
    TableOffsetEntry: [u32; 0],
}

impl rsdt {
    pub fn entry_count(&self) -> usize {
        (self.Header.Length as usize - size_of::<Self>()) / 4
    }
    pub fn entry_at(&self, id: usize) -> u32 {
        assert!(id < self.entry_count());
        unsafe {
            let p = (self as *const Self).offset(1) as *const u32;
            *(p.offset(id as isize))
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct madt {
    pub Header: header,
    pub LapicAddress: u32,
    pub Flags: u32,
    Table: [u32; 0],
}

impl Checkable for madt {
    fn check(&self) -> bool {
        &self.Header.Signature == b"APIC" && self.Header.Length >= size_of::<Self>() as u32
    }
}

#[derive(Debug)]
pub enum MadtEntry {
    Unknown(MadtEntry_Unknown),
    LocalApic(MadtEntry_LocalApic),
    IoApic(MadtEntry_IoApic),
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntry_Unknown {
    pub Type: u8,
    pub Length: u8,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntry_LocalApic {
    pub Type: u8,   // 0
    pub Length: u8,
    pub ProcessorId: u8,
    pub Id: u8,
    pub LapicFlags: u32,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntry_IoApic {
    pub Type: u8,   // 1
    pub Length: u8,
    pub Id: u8,
    pub Reserved: u8,
    pub Address: u32,
    pub GlobalIrqBase: u32,
}

#[derive(Debug)]
pub struct MadtEntryIter<'a> {
    madt: &'a madt,
    ptr: *const u8,
    end_ptr: *const u8,
}

impl madt {
    pub fn entry_iter(&self) -> MadtEntryIter {
        let ptr = unsafe{ (self as *const Self).offset(1) } as *const u8;
        let end_ptr = unsafe{ ptr.offset(self.Header.Length as isize) };
        MadtEntryIter { madt: self, ptr, end_ptr }
    }
}

impl<'a> Iterator for MadtEntryIter<'a> {
    type Item = MadtEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr >= self.end_ptr {
            return None;
        }
        unsafe {
            let typeId = *self.ptr.offset(0);
            let len = *self.ptr.offset(1);
            let ret = Some(match typeId {
                0 => MadtEntry::LocalApic( (&*(self.ptr as *const MadtEntry_LocalApic)).clone() ),
                1 => MadtEntry::IoApic( (&*(self.ptr as *const MadtEntry_IoApic)).clone() ),
                _ => MadtEntry::Unknown( (&*(self.ptr as *const MadtEntry_Unknown)).clone() ),
            });        
            self.ptr = self.ptr.offset(len as isize);
            ret       
        }
    }
}