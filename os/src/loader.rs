use crate::{config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE}, trap::context::TrapContext, utils::Address};


#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE]
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE]
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [
    KernelStack{ data: [0; KERNEL_STACK_SIZE] };
    MAX_APP_NUM
];

static USER_STACK: [UserStack; MAX_APP_NUM] = [
    UserStack{ data: [0; USER_STACK_SIZE] };
    MAX_APP_NUM
];

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push<T>(&self, data: T) -> usize {
        let data_ptr = (self.get_sp() - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *data_ptr = data;
            data_ptr as usize
        }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

pub fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

#[allow(unused)]
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        safe fn _num_app();
    }
    Address::<usize>::new(_num_app as usize).read()
}

pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push(TrapContext::init(
        get_base_i(app_id), USER_STACK[app_id].get_sp()
    ))
}