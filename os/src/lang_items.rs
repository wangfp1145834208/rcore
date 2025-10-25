use core::{panic::PanicInfo};

use crate::{error, sbi::shutdown, stack_trace::print_stack_trace, warn};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        warn!("Panicked: {}", info.message());
    }
    unsafe {
        print_stack_trace();
    }
    shutdown(true)
}