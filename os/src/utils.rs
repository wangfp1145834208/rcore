use core::{cell::UnsafeCell, marker::PhantomData};

pub struct Address<T>{
    addr: UnsafeCell<usize>,
    _phantom: PhantomData<T>,
}

impl <T> Address<T> {
    pub fn new(addr: usize) -> Self {
        Self {
            addr: UnsafeCell::new(addr),
            _phantom: PhantomData,
        }
    }

    pub fn get_addr(&self) -> *const T {
        unsafe {
            *(self.addr.get() as *const usize) as *const T
        }
    }

    pub fn add(&self, offset: usize) -> &Self {
        let new_addr = unsafe {
            self.get_addr().add(offset) as usize
        };
        unsafe {
            *self.addr.get() = new_addr;
        }
        self
    }

    pub fn read(&self) -> T {
        unsafe {
            self.get_addr().read_volatile()
        }
    }
}