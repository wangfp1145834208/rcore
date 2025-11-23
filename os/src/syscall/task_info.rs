use core::ptr::addr_of;

use crate::{mm::{user_data_copy}, task::{current_user_token, get_task_info}};

// 该方法在开启虚拟地址后肯会有问题
pub fn sys_task_info(id: usize, ts: *mut os_common::TaskInfo) -> isize {
    if let Some(task_info) = get_task_info(id) {
        user_data_copy(current_user_token(), ts.cast::<u8>(), addr_of!(task_info).cast::<u8>(), core::mem::size_of::<os_common::TaskInfo>());
        0
    } else {
        -1
    }
}
