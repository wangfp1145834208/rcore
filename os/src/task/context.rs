use crate::{trap::trap_return};

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

    pub fn goto_trap_return(kstack: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack,
            s: [0usize; 12]
        }
    }
}
