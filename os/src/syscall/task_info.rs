use crate::task::{get_task_info};

// 该方法在开启虚拟地址后肯会有问题
pub fn sys_task_info(id: usize, ts: *mut os_common::TaskInfo) -> isize {
    get_task_info(id, ts)
}
