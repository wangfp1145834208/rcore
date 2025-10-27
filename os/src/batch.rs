use core::{arch::asm, cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit};

use crate::{info, kernel, sbi::shutdown, trap::context::TrapContext, utils::Address};

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

struct AppInfo {
    start: usize,
    end: usize,
    name: &'static str,
}

impl AppInfo {
    fn app_len(&self) -> usize {
        self.end - self.start
    }

    fn app_src(&self) -> &'static [u8] {
        unsafe {
            core::slice::from_raw_parts(self.start as *const u8, self.app_len())        
        }
    }
}

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_info: [MaybeUninit<AppInfo>; MAX_APP_NUM],
}

impl AppManager {
    fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            info!("All applications completed!");
            shutdown(false);
        }
        info!("[kernel] loading app_{}", app_id);
        unsafe {
            (APP_BASE_ADDRESS..(APP_BASE_ADDRESS+APP_SIZE_LIMIT)).for_each(|pa| {
                (pa as *mut usize).write_volatile(0);
            });

            let app_info = self.app_info[app_id].assume_init_ref();
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_info.app_len());
            app_dst.copy_from_slice(app_info.app_src());
            asm!("fence.i");
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

    fn display_app_info(&self, app_id: usize) {
        let app_info = unsafe {
            self.app_info[app_id].assume_init_ref()
        };
        kernel!(
            "[kernel] app_{} [{:0x} {:0x}) - {}",
            app_id,
            app_info.start,
            app_info.end,
            app_info.name
        );
    }

    pub fn print_app_info(&self) {
        info!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            self.display_app_info(i);
        }
    }

    pub fn print_cur_app_info(&self) {
        // 在load_app后，会紧跟着move_to_next_app
        self.display_app_info(self.current_app-1);
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            init_app_manager()
        })
    };
}

unsafe fn init_app_manager() -> AppManager {
    unsafe extern "C" {
        safe fn _num_app();
    }
    let app_info_addr = Address::<usize>::new(_num_app as usize);
    let num_app = app_info_addr.read();
    if num_app > MAX_APP_NUM {
        panic!("to many apps: {num_app} > {MAX_APP_NUM}")
    }

    let mut app_infos: [MaybeUninit<AppInfo>; MAX_APP_NUM] = unsafe {MaybeUninit::uninit().assume_init()};
    app_info_addr.add(1);
    for i in 0..num_app {
        let app_start = app_info_addr.read();
        let app_name_addr = Address::<u8>::new(app_info_addr.add(1).read());
        let app_name_len = app_name_addr.read() as usize;
        let app_name = unsafe {
            core::str::from_raw_parts(app_name_addr.add(1).get_addr(), app_name_len)
        };
        let app_end = app_info_addr.add(1).read();
        app_infos[i] = MaybeUninit::new(AppInfo{
            start: app_start,
            end: app_end,
            name: app_name,
        });
    }

    AppManager { num_app, current_app: 0, app_info: app_infos }
}

pub fn init() {
    print_app_info();
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_accese().print_app_info();
}

pub fn print_cur_app_info() {
    APP_MANAGER.exclusive_accese().print_cur_app_info();
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
