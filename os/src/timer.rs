use riscv::register::time;

use crate::{config::CLOCK_FREQ, sbi::set_timer};

const MSEC_PER_SEC: usize = 1_000;
const USEC_PER_SEC: usize = 1_000_000;
const TICKS_PER_SEC: usize = 100;

pub fn get_time() -> usize {
    time::read()
}

// 此处无法写成get_time() / CLOCK_FREQ * MSEC_PER_SEC。因为已有精度损失
pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / USEC_PER_SEC)
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
