use super::kernelvm::*;
use crate::sync::SpinLock as Mutex;
use alloc::string::*;
use alloc::sync::Arc;
use alloc::vec::*;

pub struct ModuleSymbol {
    pub name: String,
    pub loc: usize,
}
pub struct ModuleDependence {
    pub name: String,
    pub api_version: i32,
}
pub struct ModuleInfo {
    pub name: String,
    pub version: i32,
    pub api_version: i32,
    pub exported_symbols: Vec<String>,
    pub dependent_modules: Vec<ModuleDependence>,
}

impl ModuleInfo {
    pub fn parse(input: &str) -> Option<ModuleInfo> {
        let lines: Vec<&str> = input.split('\n').collect();
        let mut minfo = ModuleInfo {
            name: String::from("<anonymous module>"),
            version: 0,
            api_version: 0,
            exported_symbols: Vec::new(),
            dependent_modules: Vec::new(),
        };
        for l in lines {
            if l.len() == 0 {
                continue;
            }
            let columns: Vec<&str> = l.split(':').collect();
            if columns.len() != 2 {
                return None;
            }
            match columns[0] {
                "name" => {
                    minfo.name = String::from(columns[1]);
                }
                "version" => {
                    minfo.version = columns[1].parse::<i32>().unwrap();
                }
                "api_version" => {
                    minfo.api_version = columns[1].parse::<i32>().unwrap();
                }
                "exported_symbols" => {
                    let symbols: Vec<&str> = columns[1].split(",").collect();
                    minfo.exported_symbols = symbols.iter().map(|s| String::from(*s)).collect();
                }
                "dependence" => {
                    let dependences: Vec<&str> = columns[1].split(",").collect();
                    for dep in dependences.iter() {
                        if dep.len() == 0 {
                            continue;
                        }
                        let pair: Vec<&str> = dep.split("=").collect();

                        minfo.dependent_modules.push(ModuleDependence {
                            name: String::from(pair[0]),
                            api_version: pair[1].parse::<i32>().unwrap(),
                        });
                    }
                }
                _ => {
                    return None;
                }
            }
        }
        Some(minfo)
    }
}

pub enum ModuleState {
    Ready,
    PrepareUnload,
    Unloading,
}

pub struct ModuleRef;
pub struct LoadedModule {
    pub info: ModuleInfo,
    pub exported_symbols: Vec<ModuleSymbol>,
    pub used_counts: i32,
    pub using_counts: Arc<ModuleRef>,
    pub vspace: VirtualSpace,
    pub lock: Mutex<()>,
    pub state: ModuleState,
}

impl LoadedModule {
    // Grabs a reference to the kernel module.
    // For example, a file descriptor to a device file controlled by the module is a reference.
    // This must be called without the lock!
    pub fn grab(&self) -> Arc<ModuleRef> {
        Arc::clone(&self.using_counts)
    }
}

// Equivalent of Linux kobject. Has a reference counter to module
pub struct KObject {
    pub name: String,
}
