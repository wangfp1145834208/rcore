use crate::{info, println, syscall::{fs::sys_write, process::{sys_exit, sys_get_time, sys_yield}, task_info::sys_task_info}, task::metric_sys_call};

pub mod fs;
pub mod process;
pub mod task_info;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    metric_sys_call(syscall_id);
    match syscall_id {
        os_common::SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        os_common::SYSCALL_EXIT => sys_exit(args[0]),
        os_common::SYSCALL_YIELD => sys_yield(),
        os_common::SYSCALL_TASK_INFO => sys_task_info(args[0], args[1] as *mut os_common::TaskInfo),
        os_common::SYSCALL_GET_TIME => sys_get_time(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

#[unsafe(no_mangle)]
pub fn test_syscall_list() {
    let call = os_common::syscall_list();
    for c in &call {
        info!("call_id: {}", c.id);
    }
    println!("syscall list test passed");
}
