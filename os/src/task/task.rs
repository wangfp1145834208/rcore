use core::fmt::Display;

use os_common::{syscall_list};

use crate::{info, kernel, loader::{get_base_i, init_app_cx}, task::context::TaskContext, utils::Address};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub cx: TaskContext,

    app_range: (usize, usize),
    pub app_name: &'static str,

    pub user_time: usize,
    pub kernel_time: usize,

    pub sys_call: [os_common::SyscallInfo; os_common::MAX_SYSCALL_NUM],
}

impl TaskControlBlock {
    pub fn set_app_info(&mut self, app_info_addr: &mut Address<usize>) {
        let app_start = app_info_addr.read();
        let app_name_addr = Address::<u8>::new(app_info_addr.add(1).read());
        let app_len = app_name_addr.read() as usize;
        self.app_name = unsafe {
            core::str::from_raw_parts(app_name_addr.add(1).get_addr(), app_len)
        };
        self.app_range = (app_start, app_info_addr.add(1).read());
    }

    // 在加载前完成内存初始化
    pub fn set_to_ready(&mut self, app_id: usize) {
        if self.app_range == (0, 0) || self.app_range.0 > self.app_range.1 {
            panic!("[kernel] invalid app range: [{}, {})", self.app_range.0, self.app_range.1);
        }

        
        let dst_addr = get_base_i(app_id);
        let size = self.app_range.1.wrapping_sub(self.app_range.0);
        unsafe {
            let src = core::slice::from_raw_parts(self.app_range.0 as *const u8, size);
            let dst = core::slice::from_raw_parts_mut(dst_addr as *mut u8, size);
            dst.copy_from_slice(src);
        }
        self.cx = TaskContext::ret_to_restore(init_app_cx(app_id));
        self.status = TaskStatus::Ready;
        info!("[kernel] load app {} from (.data)[0x{:0x}, 0x{:0x}) to kernel (.text)[0x{:0x}, 0x{:0x})", self.app_name, self.app_range.0, self.app_range.1, dst_addr, dst_addr+size);
    }

    pub fn set_status(&mut self, status: TaskStatus) -> *const TaskContext {
        self.status = status;
        if status == TaskStatus::Exited {
            kernel!("app {} exit with user_time: {}us, kernel_time: {}us", self.app_name, self.user_time, self.kernel_time);
        }
        &self.cx as *const TaskContext
    }

    pub fn update_user_time(&mut self, duration: usize) {
        self.user_time += duration;
    }

    pub fn update_kernel_time(&mut self, duration: usize) {
        self.kernel_time += duration;
    }

    pub fn metric_sys_call(&mut self, id: usize) {
        match self.sys_call.iter_mut()
                    .find(|call| call.id == id) {
            Some(call) => {call.times += 1},
            None => {},
        };
    }
}

impl Default for TaskControlBlock {
    fn default() -> Self {
        Self {
            status: TaskStatus::UnInit,
            cx: TaskContext::zero_init(),

            app_range: (0, 0),
            app_name: "",

            user_time: 0,
            kernel_time: 0,
            
            sys_call: syscall_list(),
        }
    }
}

impl Display for TaskControlBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "app_name: {}, range: [0x{:0x}, 0x{:0x}), status: {:?}", self.app_name, self.app_range.0, self.app_range.1, self.status)        
    }
}
