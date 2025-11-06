#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]

extern crate alloc;

#[path = "boards/qemu.rs"]
mod board;

mod lang_items;
mod sbi;
mod console;
pub(crate) mod logging;
pub(crate) mod sync;
// pub(crate) mod batch;
pub(crate) mod config;
pub(crate) mod trap;
pub(crate) mod syscall;
pub(crate) mod stack_trace;
pub(crate) mod loader;
pub(crate) mod task;
pub(crate) mod timer;
pub(crate) mod utils;
pub(crate) mod mm;

use core::{arch::global_asm};

use crate::{board::MEMORY_END, mm::{address::vpn_range_test, frame_allocator::frame_allocator_test, heap_allocator::heap_test, memory_set::remap_test}, sbi::shutdown, syscall::test_syscall_list, trap::check_kernel_interrupt};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    info!("[kernel] hello, rcore");
    log_level(true);
    log_sections();
    // heap和frame_allocator分配的都是内核的地址空间，为identical
    mm::init_heap();
    mm::init_frame_allocator();
    mm::KERNEL_SPACE.exclusive_access().activate();

    if let Some(_) = option_env!("TEST") {
        main_test();
        shutdown(false);
    }

    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();

    unsafe { riscv::register::sstatus::set_sie(); }
    loop {
        if check_kernel_interrupt() {
            kernel!("kernel interrupt returned.");
            break;
        }
    }
    unsafe { riscv::register::sstatus::clear_sie(); }

    task::run_first_task();
    panic!("Unreachable in rust_main!");
}

#[inline(never)]
fn clear_bss() {
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe {
            (a as *mut u8).write_volatile(0);
        }
    });
}

fn log_sections() {
    unsafe extern "C" {
        safe fn stext();
        safe fn etext();

        safe fn sdata();
        safe fn edata();

        safe fn srodata();
        safe fn erodata();

        safe fn sbss();
        safe fn ebss();

        safe fn ekernel();
    }

    kernel!(".text range: {:#x} - {:#x}", stext as usize, etext as usize);
    kernel!(".rodata range: {:#x} - {:#x}", srodata as usize, erodata as usize);
    kernel!(".data range: {:#x} - {:#x}", sdata as usize, edata as usize);
    kernel!(".bss range: {:#x} - {:#x}", sbss as usize, ebss as usize);
    kernel!(".frame range: {:#x} - {:#x}", ekernel as usize, MEMORY_END as usize);
}

fn log_level(on: bool) {
    if on {
        error!("error");
        warn!("warn");
        info!("info");
        debug!("debug");
        trace!("trace");
    }
}

fn main_test() {
    test_syscall_list();
    heap_test();
    frame_allocator_test();
    vpn_range_test();
    remap_test();
}
