use crate::lkm::manager::ModuleManager;
use crate::sync::Mutex;
use crate::syscall::{check_and_clone_cstr, SysResult, Syscall};
use alloc::collections::btree_map::BTreeMap;
use compression::prelude::Action;

impl Syscall<'_> {
    pub fn sys_init_module(
        &mut self,
        module_image: *const u8,
        len: usize,
        param_values: *const u8,
    ) -> SysResult {
        let mut proc = self.process();
        let modimg = unsafe { self.vm().check_read_array(module_image, len)? };
        let copied_param_values = check_and_clone_cstr(param_values)?;

        ModuleManager::with(|kmm| kmm.init_module(modimg, &copied_param_values))
    }

    pub fn sys_delete_module(&mut self, module_name: *const u8, flags: u32) -> SysResult {
        let mut proc = self.process();
        let copied_modname = check_and_clone_cstr(module_name)?;
        info!("[LKM] Removing module {:?}", copied_modname);
        let ret = ModuleManager::with(|kmm| kmm.delete_module(&copied_modname, flags));
        info!("[LKM] Remove module {:?} done!", copied_modname);
        ret
    }
}
