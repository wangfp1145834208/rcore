use crate::{mm, task::{exit_current_and_run_next, suspend_current_and_run_next}, timer::get_time_ms, warn};

pub fn sys_exit(exit_code: usize) -> ! {
    warn!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("unreachable")
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_mem_apply(va: usize, size: usize) -> isize {
    if let Some(apply_size) = mm::mem_apply(va.into(), size) {
        apply_size as isize
    } else {
        -1
    }
}
