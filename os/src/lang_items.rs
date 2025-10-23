use core::{panic::PanicInfo};

use crate::{error, warn, sbi::shutdown};

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
    shutdown(true)
}