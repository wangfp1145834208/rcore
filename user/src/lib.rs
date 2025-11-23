#![no_std]
#![feature(linkage)]
#![allow(static_mut_refs)]

mod lang_items;
#[macro_use]
pub mod console;
mod syscall;
mod allocator;

pub use syscall::sys_task_info;
pub use allocator::get_heap_addr;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bss();
    init_heap();
    exit(main());
    panic!("unreachable")
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

use crate::{allocator::init_heap, syscall::{sys_exit, sys_get_time, sys_write, sys_yield}};

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn get_task_info(app_id: usize) -> Option<os_common::TaskInfo> {
    let mut ts = os_common::TaskInfo::default();
    let result = sys_task_info(app_id, (&mut ts) as *mut os_common::TaskInfo);
    if result == 0 {
        Some(ts)
        // println!("task_info - {} - {}", app_id, ts);
    } else {
        None
        // println!("task_info - {} - get task_info failed", app_id);
    }
}
