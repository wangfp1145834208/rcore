#![no_std]
#![no_main]

use user_lib::{get_time, task_info, yield_};

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    task_info();
    let current_time = get_time();
    let wait_for = current_time + 1;
    while get_time() < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    0
}
