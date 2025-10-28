use core::{arch::global_asm, sync::atomic::{Ordering, AtomicBool}};

use riscv::register::{scause::{self, Exception, Interrupt, Trap}, sie, sstatus, stval, stvec};

use crate::{kernel, syscall::syscall, task::{exit_current_and_run_next, metric_kernel_time, metric_user_time, suspend_current_and_run_next}, timer::set_next_trigger, trap::context::TrapContext, warn};

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

static KERNEL_INTERRUPT_TRIGGERED: AtomicBool = AtomicBool::new(false);

pub fn check_kernel_interrupt() -> bool {
    KERNEL_INTERRUPT_TRIGGERED.load(Ordering::Acquire)
}

pub fn trigger_kernel_interrupt() {
    KERNEL_INTERRUPT_TRIGGERED.store(true, Ordering::Release);
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    match sstatus::read().spp() {
        sstatus::SPP::Supervisor => kernel_trap_handler(cx),
        sstatus::SPP::User => user_trap_handler(cx)
    }
}

pub fn kernel_trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            kernel!("kernel interrupt: from timer");
            trigger_kernel_interrupt();
            set_next_trigger();
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            panic!("[kernel] PageFault in kernel, bad addr={:0x}, bad instruction={:#x}, kernel killed it", stval, cx.sepc);
        }
        _ => {
            panic!("unknown kernel exception or interrupt");
        }
    }
    cx
}

pub fn user_trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    // 从用户态切回内核态
    metric_user_time();

    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        },
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            warn!("[kernel] PageFault in application, kernel killed it.");
            exit_current_and_run_next();
        },
        Trap::Exception(Exception::IllegalInstruction) => {
            warn!("[kernel] IllegalIstruction in application, kernel killed it.");
            exit_current_and_run_next();
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    // 切回至用户态
    metric_kernel_time();
    cx
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}
