use core::arch::asm;

// const SYSCALL_WRITE: usize = 64;
// const SYSCALL_EXIT: usize = 93;
// const SYSCALL_YIELD: usize = 124;
// const SYSCALL_GET_TIME: usize = 169;
// const SYSCALL_TASK_INFO: usize = 256;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") args[0] => ret,
            in("a1") args[1],
            in("a2") args[2],
            in("a7") id
            // inlateout("x10") args[0] => ret,
            // in("x11") args[1],
            // in("x12") args[2],
            // in("x17") id
        );
    }
    ret
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(os_common::SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> isize {
    syscall(os_common::SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(os_common::SYSCALL_YIELD, [0; 3])
}

pub fn sys_get_time() -> isize {
    syscall(os_common::SYSCALL_GET_TIME, [0; 3])
}

pub fn sys_task_info(app_id: usize, ts: *mut os_common::TaskInfo) -> isize {
    syscall(os_common::SYSCALL_TASK_INFO, [app_id, ts as usize, 0])
}
