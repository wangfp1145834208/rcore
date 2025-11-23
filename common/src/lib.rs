#![no_std]

use core::fmt::Display;

pub const MAX_SYSCALL_NUM: usize = 16;
pub const MAX_APP_NAME_LEN: usize = 32;

#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct CString<const LEN: usize>{
    inner: [u8; LEN],
    len: usize,
}

impl <const LEN: usize> CString<LEN> {
    pub fn write(&mut self, s: impl AsRef<[u8]>) {
        let input = s.as_ref();
        let len = input.len().min(LEN-1);
        self.len = len;
        self.inner[..len].copy_from_slice(&input[..self.len]);
    }   

    pub fn len(&self) -> usize {
        self.len
    }
}

impl <const LEN: usize> Display for CString<LEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", core::str::from_utf8(&self.inner[..self.len]).unwrap())
    }
}

impl <const LEN: usize> Default for CString<LEN> {
    fn default() -> Self {
        Self {
            inner: [0u8; LEN],
            len: 0
        }
    }
}

#[repr(C, align(8))]
#[derive(Default, Clone, Copy)]
pub struct SyscallInfo {
    pub id: usize,
    pub times: usize,
}

impl From<usize> for SyscallInfo {
    fn from(id: usize) -> Self {
        Self { id, times: 0 }
    }
}

#[repr(C, align(8))]
#[derive(Default)]
pub struct TaskInfo {
    pub id: usize,
    pub name: CString<MAX_APP_NAME_LEN>,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub time: usize
}

impl Display for TaskInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}(app_id:{}) - call(", self.name, self.id)?;
        self.call.iter()
            .take_while(|&c| c.id != 0)
            .for_each(|c| {
                write!(f, "{}:{},", syscall_desc(c.id), c.times).unwrap();
            });
        write!(f, ") - time:{}us", self.time)
    }
}

macro_rules! init_syscall {
    ( $( ($name:ident, $val:expr, $desc:expr) )+ ) => {
        $(
            pub const $name: usize = $val;
        )+

        #[allow(unused_assignments)]
        pub fn syscall_list() -> [SyscallInfo; MAX_SYSCALL_NUM] {
            let mut call = [SyscallInfo::default(); MAX_SYSCALL_NUM];
            let mut idx = 0;
            $(
                call[idx] = $val.into();
                idx += 1;
            )+

            call
        }

        fn syscall_desc(val: usize) -> &'static str {
            match val {
                $(
                    $val => $desc,
                )+
                _ => ""
            }
        }
    };
}

init_syscall! {
    (SYSCALL_WRITE, 64, "write")
    (SYSCALL_EXIT, 93, "exit")
    (SYSCALL_YIELD, 124, "yield")
    (SYSCALL_GET_TIME, 169, "get_time")
    (SYSCALL_TASK_INFO, 410, "task_info")
}
