use core::{arch::{asm, global_asm}, ptr::addr_of, sync::atomic::{AtomicBool, Ordering}};

use riscv::register::{scause::{self, Exception, Interrupt, Trap}, sie, sstatus, stval, stvec};

use crate::{config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT}, info, kernel, mm::KERNEL_SPACE, println, syscall::syscall, task::{current_trap_cx, current_user_token, exit_current_and_run_next, metric_kernel_time, metric_user_time, suspend_current_and_run_next}, timer::set_next_trigger, trap::context::TrapContext, warn};

pub mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
    set_user_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe extern "C" {
        safe fn __kernel_trap();
    }
    unsafe {
        stvec::write(__kernel_trap as usize, stvec::TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, stvec::TrapMode::Direct);
    }
}

static KERNEL_INTERRUPT_TRIGGERED: AtomicBool = AtomicBool::new(false);

pub fn check_kernel_interrupt() -> bool {
    KERNEL_INTERRUPT_TRIGGERED.load(Ordering::Acquire)
}

pub fn trigger_kernel_interrupt() {
    KERNEL_INTERRUPT_TRIGGERED.store(true, Ordering::Release);
}

// 这里使用页底作为栈顶
pub fn get_trap_cx_sp() -> usize {
    TRAP_CONTEXT + PAGE_SIZE- core::mem::size_of::<TrapContext>()
}

/*
:@parameter cx: 用户trap时不需要cx参数（因为位置固定）；内核trap时需要cx参数
:@return:
1. TrapContext的地址
2. satp
*/
#[unsafe(no_mangle)]
pub fn trap_handler(cx: TrapContext) -> (usize, usize) {
    // 设置为内核trap入口（__all_traps的原始地址）
    set_kernel_trap_entry();
    match sstatus::read().spp() {
        sstatus::SPP::Supervisor => kernel_trap_handler(&cx), // 传借用，保证地址不变
        sstatus::SPP::User => user_trap_handler()
    }
}

fn kernel_trap_handler(cx: &TrapContext) -> (usize, usize) {
    info!("kernel trap");
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            kernel!("kernel interrupt: from timer");
            trigger_kernel_interrupt();
            set_next_trigger();
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            panic!("[kernel] PageFault in kernel, bad addr={:#x}, bad instruction={:#x}, kernel killed it", stval, cx.sepc);
        }
        cause => {
            panic!("unknown kernel exception or interrupt: {:?} at {:#x}", cause, stval);
        }
    }
    (addr_of!(cx) as usize, KERNEL_SPACE.exclusive_access().token())
}

fn user_trap_handler() -> (usize, usize) {
    // 从用户态切回内核态
    metric_user_time();

    let cx = current_trap_cx();
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
    // 设置trap入口为TRAMPOLINE
    set_user_trap_entry();

    (get_trap_cx_sp(), current_user_token())
}

pub fn trap_return() -> ! {
    // 0xfffffffffffff082 
    set_user_trap_entry();
    unsafe extern "C" {
        safe fn __all_traps();
        safe fn __restore();
    }
    let restore_va = TRAMPOLINE + __restore as usize - __all_traps as usize;
    unsafe {
        asm!{
            "fence.i",
            "jr {va}",
            va = in(reg) restore_va,
            in("a0") get_trap_cx_sp(),
            in("a1") current_user_token(),
            options(noreturn)
        };
    }
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}

#[allow(unused)]
pub fn kernel_trap_test() {
    set_kernel_trap_entry();
    unsafe {
        riscv::register::sstatus::set_sie();
    }
    loop {
        if check_kernel_interrupt() {
            info!("kernel interrupt returend.");
            break;
        }
    }
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
    set_user_trap_entry();
    println!("kernel interrupt test passed!");
}
