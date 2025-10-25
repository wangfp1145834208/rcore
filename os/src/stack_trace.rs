use core::arch::asm;

use crate::{debug};

pub unsafe fn print_stack_trace() {
    let mut fp: *const usize;
    unsafe {
        asm!("mv {}, fp", out(reg) fp);
    }

    debug!("== Begin stack trace ==");
    while fp != core::ptr::null() {
        let save_ra = unsafe {*fp.sub(1)};
        let save_fp = unsafe {*fp.sub(2)};

        debug!("ra={:0x}, fp={:0x}", save_ra, save_fp);

        fp = save_fp as *const usize;
    }
    debug!("== End stack trace ==");
}
