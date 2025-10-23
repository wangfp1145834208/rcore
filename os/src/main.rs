#![no_std]
#![no_main]
mod lang_items;
mod sbi;
mod console;
pub(crate) mod logging;
pub(crate) mod sync;
pub(crate) mod batch;

use core::{arch::global_asm};

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    info!("hello, rcore");
    log_level(true);
    log_sections();
    panic!("shutdown");
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
    }

    info!("text range: {:0x} - {:0x}", stext as usize, etext as usize);
    info!("rodata range: {:0x} - {:0x}", srodata as usize, erodata as usize);
    info!("data range: {:0x} - {:0x}", sdata as usize, edata as usize);
    info!("bss range: {:0x} - {:0x}", sbss as usize, ebss as usize);
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
