use core::{panic::PanicInfo};

use crate::println;

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    let err = panic_info.message();
    if let Some(location) = panic_info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("Panicked: {}", err);
    }
    unreachable!()
}
