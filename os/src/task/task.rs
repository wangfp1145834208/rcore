use core::fmt::Display;

use os_common::{syscall_list};

use crate::{config::{TRAP_CONTEXT, kernel_stack_position}, info, kernel, mm::{KERNEL_SPACE, PhysPageNum, VirtAddr, memory_set::{MapPermission, MemorySet}}, task::context::TaskContext, trap::{context::TrapContext, trap_handler}, utils::Address};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub cx: TaskContext,

    pub memory_set: MemorySet,
    /*
    * app的TrapContext所在的物理页
    * 因为应用被分配的所有frame都是位于内核中（ekernel ~ MEMORY_END），
    * 且内核数据使用的是Identical映射方式，所以可以通过trap_cx_ppn直接读取对应位置的数据
    */
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,

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

        let elf = self.get_app_data();
        let (user_sp, entry_point) = self.memory_set.load_elf(elf);
        let trap_cx_ppn = self.memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access()
            .insert_framed_area(
                kernel_stack_bottom.into(), 
                kernel_stack_top.into(), 
                MapPermission::W | MapPermission::R,
            );
        
        self.trap_cx_ppn = trap_cx_ppn;
        self.base_size = user_sp;
        self.cx = TaskContext::goto_trap_return(kernel_stack_top);
        self.status = TaskStatus::Ready;

        let trap_cx = self.get_trap_context();
        *trap_cx = TrapContext::init(
            entry_point, 
            user_sp, 
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        info!("[kernel] start app {} from (.data)[{:#x}, {:#x})", self.app_name, self.app_range.0, self.app_range.1);
    }

    fn app_size(&self) -> usize {
        self.app_range.1.wrapping_sub(self.app_range.0)
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

    fn get_app_data(&self) -> &'static [u8] {
        unsafe {
            core::slice::from_raw_parts(self.app_range.0 as *const u8, self.app_size())
        }
    }

    // 我这里将TrapContext作为栈来使用，所以放在了page的最后部分
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut_from_back::<TrapContext>()
    }
}

impl Default for TaskControlBlock {
    fn default() -> Self {
        Self {
            status: TaskStatus::UnInit,
            cx: TaskContext::zero_init(),
            memory_set: MemorySet::new_bare(),
            trap_cx_ppn: 0.into(),
            base_size: 0,

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
