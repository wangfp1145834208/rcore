#[derive(Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0usize; 12]
        }
    }

    pub fn ret_to_restore(kstack: usize) -> Self {
        unsafe extern "C" {
            safe fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack,
            s: [0usize; 12]
        }
    }
}
