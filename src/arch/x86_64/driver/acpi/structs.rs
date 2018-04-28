// Reference: xv6-x86_64 acpi.h
// Copy from crate 'acpica-sys'

use util::{Checkable, bytes_sum};
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [i8; 6],
    pub revision: u8,
    pub rsdt_physical_address: u32,
    pub length: u32,
    pub xsdt_physical_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

impl Checkable for Rsdp {
    fn check(&self) -> bool {
        &self.signature == b"RSD PTR " && bytes_sum(self) == 0
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [i8; 6],
    pub oem_table_id: [i8; 8],
    pub oem_revision: u32,
    pub asl_compiler_id: [i8; 4],
    pub asl_compiler_revision: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Rsdt {
    pub header: Header,
    table_offset_entry: [u32; 0],
}

impl Rsdt {
    pub fn entry_count(&self) -> usize {
        (self.header.length as usize - size_of::<Self>()) / 4
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
pub struct Madt {
    pub header: Header,
    pub lapic_address: u32,
    pub flags: u32,
    table: [u32; 0],
}

impl Checkable for Madt {
    fn check(&self) -> bool {
        &self.header.signature == b"APIC" && self.header.length >= size_of::<Self>() as u32
    }
}

#[derive(Debug)]
pub enum MadtEntry {
    Unknown(MadtEntryUnknown),
    LocalApic(MadtEntryLocalApic),
    IoApic(MadtEntryIoApic),
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntryUnknown {
    pub type_: u8,
    pub length: u8,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntryLocalApic {
    pub type_: u8,   // 0
    pub length: u8,
    pub processor_id: u8,
    pub id: u8,
    pub lapic_flags: u32,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MadtEntryIoApic {
    pub type_: u8,   // 1
    pub length: u8,
    pub id: u8,
    pub reserved: u8,
    pub address: u32,
    pub global_irq_base: u32,
}

#[derive(Debug)]
pub struct MadtEntryIter<'a> {
    madt: &'a Madt,
    ptr: *const u8,
    end_ptr: *const u8,
}

impl Madt {
    pub fn entry_iter(&self) -> MadtEntryIter {
        let ptr = unsafe{ (self as *const Self).offset(1) } as *const u8;
        let end_ptr = unsafe{ ptr.offset(self.header.length as isize) };
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
            let type_id = *self.ptr.offset(0);
            let len = *self.ptr.offset(1);
            let ret = Some(match type_id {
                0 => MadtEntry::LocalApic( (&*(self.ptr as *const MadtEntryLocalApic)).clone() ),
                1 => MadtEntry::IoApic( (&*(self.ptr as *const MadtEntryIoApic)).clone() ),
                _ => MadtEntry::Unknown( (&*(self.ptr as *const MadtEntryUnknown)).clone() ),
            });        
            self.ptr = self.ptr.offset(len as isize);
            ret       
        }
    }
}