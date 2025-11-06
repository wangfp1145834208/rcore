use core::ptr::{addr_of, addr_of_mut};

use buddy_system_allocator::LockedHeap;
use crate::{config::KERNEL_HEAP_SIZE, info, println};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
static DATA_VAR: [u8; 8] = [0; 8];
static mut BSS_VAR: [u8; 8] = [0; 8];

pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    info!("heap_space: {:#x}", &raw mut HEAP_SPACE as usize);
    info!("data_var: {:#x}", addr_of!(DATA_VAR) as usize);
    info!("bss_var: {:#x}", addr_of_mut!(BSS_VAR) as usize);

    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    
    let a = Box::new(5);
    assert_eq!(*a, 5);
    let a_addr = a.as_ref() as *const _ as usize;
    info!("a addr: {:#x}", a_addr);
    assert!(bss_range.contains(&a_addr));
    drop(a);

    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
