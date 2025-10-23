use core::arch::asm;

use crate::{error, info, sbi::shutdown};

use super::sync::up::UPSafeCell;
use lazy_static::*;

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x200000;

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],  // 需要记录最后一个APP片段的结束
}

impl AppManager {
    fn load_app(&self, app_id: usize) {
        if app_id > self.num_app {
            info!("All applications completed!");
            shutdown(false);
        }
        info!("[kernel] loading app_{}", app_id);
        unsafe {
            (APP_BASE_ADDRESS..(APP_BASE_ADDRESS+APP_SIZE_LIMIT)).for_each(|pa| {
                (pa as *mut usize).write_volatile(0);
            });

            let app_src = core::slice::from_raw_parts(self.app_start[app_id] as *const u8, self.app_start[app_id+1]-self.app_start[app_id]);
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT);
            app_dst.copy_from_slice(app_src);
            asm!("fence.i");
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            unsafe extern "C" {
                safe fn _num_app();
            }
            let num_app_ptr = _num_app as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM+1] = [0; MAX_APP_NUM+1];
            let app_start_raw: &[usize] = core::slice::from_raw_parts(num_app_ptr.add(1), num_app+1);
            app_start[..=num_app].copy_from_slice(app_start_raw);

            AppManager { num_app, current_app: 0, app_start }
        })
    };
}
