#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_task_info, yield_};

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Get task info!");

    print_task_info(0);
    yield_();
    print_task_info(3);
    print_task_info(6);
    0
}

fn print_task_info(app_id: usize) {
    if let Some(ts) = get_task_info(app_id) {
        println!("[task_info - {}]{}", app_id, ts);
    } else {
        println!("[task_info - {}]get task_info failed", app_id);
    }
}
