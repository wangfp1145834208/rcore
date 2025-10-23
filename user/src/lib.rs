#![no_std]
#![feature(linkage)]

mod lang_items;
#[macro_use]
pub mod console;
mod syscall;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bss();
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

use crate::syscall::{sys_exit, sys_write};

pub fn write(fd: usize, buf: &str) -> isize {
    sys_write(fd, buf.as_bytes())
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
