use core::{fmt, sync::atomic::{Ordering, AtomicU8}};

use crate::println;

pub struct Level {
    level: u8,
    color: u8,
    tag: &'static str
}

impl Level {
    const fn new(level: u8, color: u8, tag: &'static str) -> Level {
        Level{level, color, tag}
    }
}

#[allow(unused)]
pub const KERNEL: Level = Level::new(6, 96, "KERNEL");
pub const ERROR: Level = Level::new(5, 31, "ERROR");
pub const WARN: Level  = Level::new(4, 93, "WARN");
pub const INFO: Level = Level::new(3, 34, "INFO");
pub const DEBUG: Level = Level::new(2, 32, "DEBUG");
pub const TRACE: Level = Level::new(1, 90, "TRACE");

static MIN_LEVEL: AtomicU8 = AtomicU8::new(INFO.level);

pub fn log_allow(level: &Level) -> bool {
    level.level >= MIN_LEVEL.load(Ordering::Acquire)
}

#[allow(unused)]
pub struct LogMetaInfo {
    level: &'static Level,

    file: &'static str,
    line: u32,
}

impl LogMetaInfo {
    pub fn new(level: &'static Level, file: &'static str, line: u32) -> Self {
        Self {
            level,
            file,
            line,
        }
    }
}

#[allow(unused)]
pub(crate) fn log(meta: LogMetaInfo, args: fmt::Arguments) {
    println!(
        "\u{1B}[{}m[{:>5}]({}-{}) {}\u{1B}[0m",
        meta.level.color,
        meta.level.tag,
        meta.file,
        meta.line,
        args
    )
}

pub(crate) fn init() {
    let min_level = match option_env!("LOG") {
        Some("ERROR") => ERROR.level,
        Some("WARN") => WARN.level,
        Some("INFO") => INFO.level,
        Some("DEBUG") => DEBUG.level,
        Some("TRACE") => TRACE.level,
        _ => INFO.level,
    };
    MIN_LEVEL.store(min_level, Ordering::Release);
}

#[macro_export]
macro_rules! __log {
    ($level:expr, $fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(inner $level, file!(), line!(), $fmt $(, $arg)*)
    };
    (inner $level:expr, $file:expr, $line:expr, $fmt:literal $(, $arg:expr)*) => {
        if $crate::logging::log_allow($level) {
            let meta = $crate::logging::LogMetaInfo::new(
                $level, $file, $line
            );
            $crate::logging::log(meta, format_args!($fmt $(, $arg)*));
        }
    }
}

#[macro_export]
macro_rules! kernel {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::KERNEL, $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::ERROR, $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::WARN, $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! info {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::INFO, $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::DEBUG, $fmt $(, $arg)*)
    };
}

#[macro_export]
macro_rules! trace {
    ($fmt:literal $(, $arg:expr)*) => {
        $crate::__log!(&$crate::logging::TRACE, $fmt $(, $arg)*)
    };
}
