use core::arch::global_asm;

use riscv::register::{scause::{self, Exception, Trap}, stval, stvec};

use crate::{batch::run_next_app, syscall::syscall, trap::context::TrapContext, warn};

pub mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
    unsafe extern "C" {
        safe fn __all_traps();
    }
    unsafe {
        stvec::write(__all_traps as usize, stvec::TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        },
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            warn!("[kernel] PageFault in application, kernel killed it.");
            run_next_app();
        },
        Trap::Exception(Exception::IllegalInstruction) => {
            warn!("[kernel] IllegalIstruction in application, kernel killed it.");
            run_next_app();
        },
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}
