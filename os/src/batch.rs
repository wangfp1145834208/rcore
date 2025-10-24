use core::{arch::asm};

use crate::{info, sbi::shutdown, trap::context::TrapContext};

use super::sync::up::UPSafeCell;
use lazy_static::*;

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x200000;
const USER_STACK_SIZE: usize = 4096 * 2;
const KERNAL_STACK_SIZE: usize = 4096 * 2;

#[repr(align(4096))]
pub struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

#[repr(align(4096))]
pub struct KernelStack {
    data: [u8; KERNAL_STACK_SIZE],
}

impl KernelStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNAL_STACK_SIZE
    }

    pub fn push<T>(&self, data: T) -> &'static mut T {
        let data_ptr = (self.get_sp() - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *data_ptr = data;
            data_ptr.as_mut().unwrap()
        }
    }
}

static USER_STACK: UserStack = UserStack{data: [0u8; USER_STACK_SIZE]};
static KERNEL_STACK: KernelStack = KernelStack{data: [0u8; KERNAL_STACK_SIZE]};

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],  // 需要记录最后一个APP片段的结束
}

impl AppManager {
    fn load_app(&self, app_id: usize) {
        if app_id > self.num_app {
            info!("All applications completed!");
            shutdown(false);
        }
        info!("[kernel] loading app_{}", app_id);
        unsafe {
            (APP_BASE_ADDRESS..(APP_BASE_ADDRESS+APP_SIZE_LIMIT)).for_each(|pa| {
                (pa as *mut usize).write_volatile(0);
            });

            let app_length = self.app_start[app_id+1] - self.app_start[app_id];
            let app_src = core::slice::from_raw_parts(self.app_start[app_id] as *const u8, app_length);
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_length);
            app_dst.copy_from_slice(app_src);
            asm!("fence.i");
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

    pub fn print_app_info(&self) {
        info!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            info!(
                "[kernel] app_{} [{:0x} {:0x})",
                i,
                self.app_start[i],
                self.app_start[i+1]
            )
        }
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            unsafe extern "C" {
                safe fn _num_app();
            }
            let num_app_ptr = _num_app as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM+1] = [0; MAX_APP_NUM+1];
            let app_start_raw: &[usize] = core::slice::from_raw_parts(num_app_ptr.add(1), num_app+1);
            app_start[..=num_app].copy_from_slice(app_start_raw);

            AppManager { num_app, current_app: 0, app_start }
        })
    };
}

pub fn init() {
    print_app_info();
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_accese().print_app_info();
}

pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_accese();
    let current_app = app_manager.get_current_app();
    app_manager.load_app(current_app);
    app_manager.move_to_next_app();
    drop(app_manager);

    unsafe extern "C" {
        safe fn __restore(cx: usize);
    }

    __restore(KERNEL_STACK.push(
        TrapContext::init(APP_BASE_ADDRESS, USER_STACK.get_sp())
    ) as *mut _ as usize);

    panic!("Unreachable in batch::run_next_app!")
}
