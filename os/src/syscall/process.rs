use crate::{batch::run_next_app, warn};

pub fn sys_exit(exit_code: usize) -> ! {
    warn!("[kernel] Application exited with code {}", exit_code);
    run_next_app()
}
