use crate::{batch::print_cur_app_info};

pub fn sys_task_info() -> isize {
    print_cur_app_info();
    1
}
