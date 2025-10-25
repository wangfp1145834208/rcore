#![no_std]
#![no_main]

use user_lib::{self, println};

#[unsafe(no_mangle)]
fn main() ->i32 {
    println!("Look up task info");
    user_lib::sys_task_info();
    0
}
