use super::api::*;
use super::const_reloc as loader;
use super::kernelvm::*;
use super::structs::*;
use crate::consts::*;
use crate::lkm::structs::ModuleState::{Ready, Unloading};
use crate::memory::GlobalFrameAlloc;
use crate::sync::{Condvar, SpinLock as Mutex};
use crate::syscall::SysError::*;
use crate::syscall::SysResult;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::prelude::*;
use alloc::string::*;
use alloc::sync::Arc;
use alloc::vec::*;
use core::borrow::BorrowMut;
use core::mem::transmute;
use core::slice;
use lazy_static::lazy_static;
use rcore_memory::memory_set::handler::{ByFrame, MemoryHandler};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::{Page, PAGE_SIZE};
use xmas_elf::dynamic::Tag;
use xmas_elf::program::Type::Load;
use xmas_elf::sections::SectionData;
use xmas_elf::sections::SectionData::{DynSymbolTable64, Dynamic64, Undefined};
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{
    header,
    program::{Flags, Type},
    ElfFile,
};
// The symbol data table.
global_asm!(include_str!("symbol_table.asm"));

/// Module Manager is the core part of LKM.
/// It does these jobs: Load preset(API) symbols; manage module loading dependency and linking modules.
pub struct ModuleManager {
    stub_symbols: BTreeMap<String, ModuleSymbol>,
    loaded_modules: Vec<Box<LoadedModule>>,
}

lazy_static! {
    pub static ref LKM_MANAGER: Mutex<Option<ModuleManager>> = Mutex::new(None);
}

macro_rules! export_stub {
    ($stub_name:ident) => {
        ModuleManager::create_stub_symbol(stringify!($stub_name), $stub_name as usize)
    };
}

fn neg(u: usize) -> usize {
    (-(u as i64)) as usize
}
unsafe fn write_to_addr(base: usize, offset: usize, val: usize) {
    let addr = base + offset;
    *(addr as *mut usize) = val;
}
impl ModuleManager {
    fn create_stub_symbol(symbol_name: &str, symbol_loc: usize) -> ModuleSymbol {
        ModuleSymbol {
            name: String::from(symbol_name),
            loc: symbol_loc,
        }
    }
    fn init_stub_symbols() -> BTreeMap<String, ModuleSymbol> {
        let vector: Vec<ModuleSymbol> = vec![
            export_stub!(lkm_api_pong),
            export_stub!(lkm_api_debug),
            export_stub!(lkm_api_query_symbol),
            export_stub!(lkm_api_info),
        ];
        let mut map: BTreeMap<String, ModuleSymbol> = BTreeMap::new();
        for module in vector.into_iter() {
            map.insert(module.name.clone(), module);
        }
        map
    }
    pub fn load_kernel_symbols_from_elf(&mut self) {
        extern "C" {
            fn rcore_symbol_table();
            fn rcore_symbol_table_size();
        }
        let symbol_table_start: usize = rcore_symbol_table as usize;
        let symbol_table_len: usize =
            unsafe { *(rcore_symbol_table_size as usize as *const usize) };
        info!(
            "Loading kernel symbol table {:08x} with size {:08x}",
            symbol_table_start as usize, symbol_table_len as usize
        );
        if symbol_table_len == 0 {
            info!("Load kernel symbol table failed! This is because you didn't attach kernel table onto binary.");
            return;
        }
        use compression::prelude::*;
        let zipped_symbols =
            unsafe { slice::from_raw_parts(symbol_table_start as *const u8, symbol_table_len) }
                .to_vec();

        let real_symbols = zipped_symbols
            .decode(&mut GZipDecoder::new())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        use core::slice;
        use core::str::from_utf8;
        self.init_kernel_symbols(from_utf8(&real_symbols).unwrap());
    }
    pub fn init_kernel_symbols(&mut self, kernel_symbols: &str) {
        let lines = kernel_symbols.lines();
        for l in lines.into_iter() {
            let mut words = l.split_whitespace();
            let address = words.next().unwrap();
            let stype = words.next().unwrap();
            let name = words.next().unwrap();
            // Simply add the symbol into stub.
            self.stub_symbols.insert(
                String::from(name),
                ModuleSymbol {
                    name: String::from(name),
                    loc: usize::from_str_radix(address, 16).unwrap(),
                },
            );
        }
    }
    pub fn resolve_symbol(&self, symbol: &str) -> Option<usize> {
        self.find_symbol_in_deps(symbol, 0)
    }
    fn find_symbol_in_deps(&self, symbol: &str, this_module: usize) -> Option<usize> {
        if symbol == "THIS_MODULE" {
            return Some(this_module);
        }
        if let Some(sym) = self.stub_symbols.get(symbol) {
            return Some(sym.loc);
        }

        for km in self.loaded_modules.iter().rev() {
            for sym in km.exported_symbols.iter() {
                if (&sym.name) == symbol {
                    return Some(sym.loc);
                }
            }
        }
        None
    }
    fn get_symbol_loc(
        &self,
        symbol_index: usize,
        elf: &ElfFile,
        dynsym: &[DynEntry64],
        base: usize,
        find_dependency: bool,
        this_module: usize,
    ) -> Option<usize> {
        info!("symbol index: {}", symbol_index);
        if symbol_index == 0 {
            return Some(0);
        }
        let selected_symbol = &dynsym[symbol_index];
        if selected_symbol.shndx() == 0 {
            if find_dependency {
                info!("symbol name: {}", selected_symbol.get_name(elf).unwrap());
                self.find_symbol_in_deps(selected_symbol.get_name(elf).unwrap(), this_module)
            } else {
                None
            }
        } else {
            Some(base + (selected_symbol.value() as usize))
        }
    }
    pub fn init_module(&mut self, module_image: &[u8], param_values: &str) -> SysResult {
        let elf = ElfFile::new(module_image).expect("[LKM] failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };
        if is32 {
            error!("[LKM] 32-bit elf is not supported!");
            return Err(ENOEXEC);
        }
        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {
                error!("[LKM] a kernel module must be some shared object!");
                return Err(ENOEXEC);
            }
            header::Type::SharedObject => {}
            _ => {
                error!("[LKM] ELF is not executable or shared object");
                return Err(ENOEXEC);
            }
        }
        let lkm_info = elf.find_section_by_name(".rcore-lkm").ok_or_else(|| {
            error!("[LKM] rcore-lkm metadata not found!");
            ENOEXEC
        })?;

        if let Undefined(info_content) = lkm_info.get_data(&elf).map_err(|_| {
            error!("[LKM] load rcore-lkm error!");
            ENOEXEC
        })? {
            let minfo = ModuleInfo::parse(core::str::from_utf8(info_content).unwrap()).ok_or_else(
                || {
                    error!("[LKM] parse info error!");
                    ENOEXEC
                },
            )?;
            //Check dependencies
            info!(
                "[LKM] loading module {} version {} api_version {}",
                minfo.name, minfo.version, minfo.api_version
            );
            for i in 0..self.loaded_modules.len() {
                if self.loaded_modules[i].info.name == minfo.name {
                    error!(
                        "[LKM] another instance of module {} (api version {}) has been loaded!",
                        self.loaded_modules[i].info.name, self.loaded_modules[i].info.api_version
                    );
                    return Err(EEXIST);
                }
            }
            let mut used_dependents: Vec<usize> = vec![];
            //let loaded_module_list=&mut self.loaded_modules;
            for module in minfo.dependent_modules.iter() {
                let mut module_found = false;
                for i in 0..self.loaded_modules.len() {
                    let loaded_module = &(self.loaded_modules[i]);
                    if loaded_module.info.name == module.name {
                        if loaded_module.info.api_version == module.api_version {
                            used_dependents.push(i);
                            module_found = true;
                            break;
                        } else {
                            error!("[LKM] dependent module {} found but with a different api version {}!", loaded_module.info.name, loaded_module.info.api_version);
                            return Err(ENOEXEC);
                        }
                    }
                }
                if !module_found {
                    error!("[LKM] dependent module not found! {}", module.name);
                    return Err(ENOEXEC);
                }
            }
            for module in used_dependents {
                self.loaded_modules[module].used_counts += 1;
            }
            let mut max_addr: usize;
            let mut min_addr: usize;
            let mut off_start: usize;
            max_addr = 0;
            min_addr = ::core::usize::MAX;
            off_start = 0;
            for ph in elf.program_iter() {
                if ph.get_type().unwrap() == Load {
                    if (ph.virtual_addr() as usize) < min_addr {
                        min_addr = ph.virtual_addr() as usize;
                        off_start = ph.offset() as usize;
                    }
                    if (ph.virtual_addr() + ph.mem_size()) as usize > max_addr {
                        max_addr = (ph.virtual_addr() + ph.mem_size()) as usize;
                    }
                }
            }
            max_addr += PAGE_SIZE - 1;
            max_addr &= neg(PAGE_SIZE);
            min_addr &= neg(PAGE_SIZE);
            off_start &= neg(PAGE_SIZE);
            let map_len = max_addr - min_addr + off_start;
            // We first map a huge piece. This requires the kernel model to be dense and not abusing vaddr.
            let mut vspace =
                { VirtualSpace::new(&KERNELVM_MANAGER, map_len) }.ok_or_else(|| {
                    error!("[LKM] valloc failed!");
                    ENOMEM
                })?;
            let base = vspace.start();

            //loaded_minfo.mem_start=base as usize;
            //loaded_minfo.mem_size=(map_len/PAGE_SIZE) as usize;
            //if map_len%PAGE_SIZE>0{
            //    loaded_minfo.mem_size+=1;
            //}
            {
                for ph in elf.program_iter() {
                    if ph.get_type().map_err(|_| {
                        error!("[LKM] program header error!");
                        ENOEXEC
                    })? == Load
                    {
                        let vspace_ref = &mut vspace;
                        let prog_start_addr = base + (ph.virtual_addr() as usize);
                        let prog_end_addr = prog_start_addr + (ph.mem_size() as usize);
                        let offset = ph.offset() as usize;
                        let flags = ph.flags();
                        let mut attr = MemoryAttr::default();
                        if flags.is_write() {
                            attr = attr.writable();
                        }
                        if flags.is_execute() {
                            attr = attr.execute();
                        }
                        let area_ref = vspace_ref.add_area(prog_start_addr, prog_end_addr, &attr);
                        //self.vallocator.map_pages(prog_start_addr, prog_end_addr, &attr);
                        //No need to flush TLB.
                        let target = unsafe {
                            ::core::slice::from_raw_parts_mut(
                                prog_start_addr as *mut u8,
                                ph.mem_size() as usize,
                            )
                        };
                        let file_size = ph.file_size() as usize;
                        if file_size > 0 {
                            target[..file_size]
                                .copy_from_slice(&elf.input[offset..offset + file_size]);
                        }
                        target[file_size..].iter_mut().for_each(|x| *x = 0);
                        //drop(vspace);
                    }
                }
            }

            let mut loaded_minfo = Box::new(LoadedModule {
                info: minfo,
                exported_symbols: Vec::new(),
                used_counts: 0,
                using_counts: Arc::new(ModuleRef {}),
                vspace: vspace,
                lock: Mutex::new(()),
                state: Ready,
            });
            info!(
                "[LKM] module load done at {}, now need to do the relocation job.",
                base
            );
            // We only search two tables for relocation info: the symbols from itself, and the symbols from the global exported symbols.
            let dynsym_table = {
                let elffile = &elf;
                if let DynSymbolTable64(dsym) = elffile
                    .find_section_by_name(".dynsym")
                    .ok_or_else(|| {
                        error!("[LKM] .dynsym not found!");
                        ENOEXEC
                    })?
                    .get_data(elffile)
                    .map_err(|_| {
                        error!("[LKM] corrupted .dynsym!");
                        ENOEXEC
                    })?
                {
                    dsym
                } else {
                    error!("[LKM] Bad .dynsym!");
                    return Err(ENOEXEC);
                }
            };
            info!("[LKM] Loading dynamic entry");
            if let Dynamic64(dynamic_entries) = elf
                .find_section_by_name(".dynamic")
                .ok_or_else(|| {
                    error!("[LKM] .dynamic not found!");
                    ENOEXEC
                })?
                .get_data(&elf)
                .map_err(|_| {
                    error!("[LKM] corrupted .dynamic!");
                    ENOEXEC
                })?
            {
                info!("[LKM] Iterating modules");
                // start, total_size, single_size
                let mut reloc_jmprel: (usize, usize, usize) = (0, 0, 0);
                let mut reloc_rel: (usize, usize, usize) = (0, 0, 16);
                let mut reloc_rela: (usize, usize, usize) = (0, 0, 24);
                for dent in dynamic_entries.iter() {
                    match dent.get_tag().map_err(|_| {
                        error! {"[LKM] invalid dynamic entry!"};
                        ENOEXEC
                    })? {
                        Tag::JmpRel => {
                            reloc_jmprel.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::PltRelSize => {
                            reloc_jmprel.1 = dent.get_val().unwrap() as usize;
                        }
                        Tag::PltRel => {
                            reloc_jmprel.2 = if (dent.get_val().unwrap()) == 7 {
                                24
                            } else {
                                16
                            }
                        }
                        Tag::Rel => {
                            reloc_rel.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::RelSize => {
                            reloc_rel.1 = dent.get_val().unwrap() as usize;
                        }
                        Tag::Rela => {
                            reloc_rela.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::RelaSize => {
                            reloc_rela.1 = dent.get_val().unwrap() as usize;
                        }
                        _ => {}
                    }
                }
                info!("[LKM] relocating three sections");
                let this_module = &(*loaded_minfo) as *const _ as usize;
                self.reloc_symbols(&elf, reloc_jmprel, base, dynsym_table, this_module);
                self.reloc_symbols(&elf, reloc_rel, base, dynsym_table, this_module);
                self.reloc_symbols(&elf, reloc_rela, base, dynsym_table, this_module);
                info!("[LKM] relocation done. adding module to manager and call init_module");
                let mut lkm_entry: usize = 0;
                for exported in loaded_minfo.info.exported_symbols.iter() {
                    for sym in dynsym_table.iter() {
                        if exported
                            == sym.get_name(&elf).map_err(|_| {
                                error!("[LKM] load symbol name error!");
                                ENOEXEC
                            })?
                        {
                            let exported_symbol = ModuleSymbol {
                                name: exported.clone(),
                                loc: base + (sym.value() as usize),
                            };

                            if exported == "init_module" {
                                lkm_entry = base + (sym.value() as usize);
                            } else {
                                loaded_minfo.exported_symbols.push(exported_symbol);
                            }
                        }
                    }
                }
                // Now everything is done, and the entry can be safely plugged into the vector.
                self.loaded_modules.push(loaded_minfo);
                if lkm_entry > 0 {
                    info!("[LKM] calling init_module at {}", lkm_entry);
                    unsafe {
                        LKM_MANAGER.force_unlock();
                        let init_module: fn() = transmute(lkm_entry);
                        (init_module)();
                    }
                } else {
                    error!("[LKM] this module does not have init_module()!");
                    return Err(ENOEXEC);
                }
            } else {
                error!("[LKM] Load dynamic field error!\n");
                return Err(ENOEXEC);
            }
        } else {
            error!("[LKM] metadata section type wrong! this is not likely to happen...");
            return Err(ENOEXEC);
        }
        Ok(0)
    }

    fn relocate_single_symbol(
        &mut self,
        base: usize,
        reloc_addr: usize,
        addend: usize,
        sti: usize,
        itype: usize,
        elf: &ElfFile,
        dynsym: &[DynEntry64],
        this_module: usize,
    ) {
        info!("Resolving symbol {}", sti);
        let sym_val = self
            .get_symbol_loc(sti, elf, dynsym, base, true, this_module)
            .expect("[LKM] resolve symbol failed!");
        match itype as usize {
            loader::REL_NONE => {}
            loader::REL_OFFSET32 => {
                panic!("[LKM] REL_OFFSET32 detected!")
                //    addend-=reloc_addr;
            }
            loader::REL_SYMBOLIC => unsafe {
                write_to_addr(base, reloc_addr, sym_val + addend);
            },
            loader::REL_GOT => unsafe {
                write_to_addr(base, reloc_addr, sym_val + addend);
            },
            loader::REL_PLT => unsafe {
                write_to_addr(base, reloc_addr, sym_val + addend);
            },
            loader::REL_RELATIVE => unsafe {
                write_to_addr(base, reloc_addr, base + addend);
            },
            _ => {
                panic!("[LKM] unsupported relocation type: {}", itype);
            }
        }
    }
    fn reloc_symbols(
        &mut self,
        elf: &ElfFile,
        (start, total_size, single_size): (usize, usize, usize),
        base: usize,
        dynsym: &[DynEntry64],
        this_module: usize,
    ) {
        if total_size == 0 {
            return;
        }
        for s in elf.section_iter() {
            if (s.offset() as usize) == start {
                {
                    match s.get_data(elf).unwrap() {
                        SectionData::Rela64(rela_items) => {
                            for item in rela_items.iter() {
                                let mut addend = item.get_addend() as usize;
                                let mut reloc_addr = item.get_offset() as usize;
                                let sti = item.get_symbol_table_index() as usize;
                                let itype = item.get_type() as usize;
                                self.relocate_single_symbol(
                                    base,
                                    reloc_addr,
                                    addend,
                                    sti,
                                    itype,
                                    elf,
                                    dynsym,
                                    this_module,
                                );
                            }
                        }
                        SectionData::Rel64(rel_items) => {
                            for item in rel_items.iter() {
                                let mut addend = 0 as usize;
                                let mut reloc_addr = item.get_offset() as usize;
                                let sti = item.get_symbol_table_index() as usize;
                                let itype = item.get_type() as usize;
                                self.relocate_single_symbol(
                                    base,
                                    reloc_addr,
                                    addend,
                                    sti,
                                    itype,
                                    elf,
                                    dynsym,
                                    this_module,
                                );
                            }
                        }
                        _ => {
                            panic!("[LKM] bad relocation section type!");
                        }
                    }
                }
                break;
            }
        }
    }
    pub fn delete_module(&mut self, name: &str, flags: u32) -> SysResult {
        //unimplemented!("[LKM] You can't plug out what's INSIDE you, RIGHT?");

        info!("[LKM] now you can plug out a kernel module!");
        let mut found = false;
        for i in 0..self.loaded_modules.len() {
            if &(self.loaded_modules[i].info.name) == name {
                let mut current_module = &mut (self.loaded_modules[i]);
                let mod_lock = current_module.lock.lock();
                if current_module.used_counts > 0 {
                    error!("[LKM] some module depends on this module!");
                    return Err(EAGAIN);
                }
                if Arc::strong_count(&current_module.using_counts) > 0 {
                    error!("[LKM] there are references to the module!");
                }
                let mut cleanup_func: usize = 0;
                for entry in current_module.exported_symbols.iter() {
                    if (&(entry.name)) == "cleanup_module" {
                        cleanup_func = entry.loc;
                        break;
                    }
                }
                if cleanup_func > 0 {
                    unsafe {
                        current_module.state = Unloading;
                        let cleanup_module: fn() = transmute(cleanup_func);
                        (cleanup_module)();
                    }
                } else {
                    error!("[LKM] you cannot plug this module out.");
                    return Err(EBUSY);
                }
                drop(mod_lock);

                let my_box = self.loaded_modules.remove(i);
                unsafe {
                    LKM_MANAGER.force_unlock();
                }
                //drop(mod_lock);
                found = true;
                break;
            }
        }
        if found {
            Ok(0)
        } else {
            Err(ENOENT)
        }
    }
    pub fn with<T>(f: impl FnOnce(&mut ModuleManager) -> T) -> T {
        let global_lkmm: &Mutex<Option<ModuleManager>> = &LKM_MANAGER;
        let mut locked_lkmm = global_lkmm.lock();
        let mut lkmm = locked_lkmm.as_mut().unwrap();
        f(lkmm)
    }
    pub fn init() {
        //assert_has_not_been_called!("[LKM] ModuleManager::init must be called only once");
        info!("[LKM] Loadable Kernel Module Manager loading...");
        let mut kmm = ModuleManager {
            stub_symbols: ModuleManager::init_stub_symbols(),
            loaded_modules: Vec::new(),
        };
        kmm.load_kernel_symbols_from_elf();

        //let lkmm: Mutex<Option<ModuleManager>>=Mutex::new(None);
        LKM_MANAGER.lock().replace(kmm);
        info!("[LKM] Loadable Kernel Module Manager loaded!");
    }
}
