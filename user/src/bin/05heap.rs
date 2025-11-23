#![no_std]
#![no_main]

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::vec;
use user_lib::get_heap_addr;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Heap!");
    println!("heap addr: {:#x}", get_heap_addr());

    let b = Box::new(2025usize); 
    let v = vec!(
        "05".to_owned(),
        "heap".to_owned(),
        ".rs".to_owned(),
    );

    println!("addr of box: {:#x}", b.as_ref() as *const usize as usize);
    println!("addr of vec: {:#x}", v.as_ptr() as usize);
    for s in v.iter() {
        println!("addr os str: {:#x} with value '{}'", s.as_ptr() as usize, s.as_str());
    }
    println!("Test heap OK!");

    0
}
