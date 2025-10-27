use lazy_static::lazy_static;

use crate::{info, kernel, sbi::shutdown, sync::up::UPSafeCell, task::{context::TaskContext, switch::__switch, task::TaskControlBlock}, timer::get_time_ms, utils::Address};

mod switch;
pub mod task;
pub mod context;

use task::{TaskStatus};

const MAX_APP_NUM: usize = 16;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    current_task: usize,
    tasks: [TaskControlBlock; MAX_APP_NUM],
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager::new();
}

impl TaskManager {
    fn new() -> Self {
        unsafe extern "C" {
            safe fn _num_app();
        }
        let mut app_info_addr = Address::<usize>::new(_num_app as usize);
        let num_app = app_info_addr.read();
        // 将地址切换到下一个app
        app_info_addr.add(1);

        let mut tasks = [TaskControlBlock::default(); MAX_APP_NUM];
        for app_id in 0..num_app {
            tasks[app_id].set_app_info(&mut app_info_addr);
            tasks[app_id].set_to_ready(app_id);
        };

        Self {
            num_app,
            inner: UPSafeCell::new(TaskManagerInner {
                current_task: 0,
                tasks
            })
        }
    }

    fn mark_task_status(&self, status: TaskStatus) -> *const TaskContext {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].set_status(status)
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // 这里需要全部遍历一遍，因为最后可能只剩余一个任务在运行
        (0..self.num_app).find_map(|offset| {
            let app_id = (current + offset + 1) % self.num_app;
            if inner.tasks[app_id].status == TaskStatus::Ready {
                Some(app_id)
            } else {
                None
            }
        })
    }

    fn run_first_task(&self) -> ! {
        info!("run first task");
        let mut inner = self.inner.exclusive_access();
        let current =  inner.current_task;
        let current_cx = inner.tasks[current].set_status(TaskStatus::Running);
        drop(inner);

        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut TaskContext, current_cx);
        }
        panic!("unreachable in run_first_task!");
    }

    fn run_next_task(&self, status: TaskStatus) {
        // 这里比较关键：需要在find_next_task前就重置当前任务状态，否则在只剩余一个任务的时候会出现任务无法完成的情况
        let current_cx = self.mark_task_status(status).cast_mut();
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let next_cx = inner.tasks[next].set_status(TaskStatus::Running);
            inner.current_task = next;
            drop(inner);

            unsafe {
                __switch(current_cx, next_cx);
            }
        } else {
            kernel!("All application completed! Total cost {}ms", get_time_ms());
            shutdown(false);
        }
    }

    fn print_task_info(&self) {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        kernel!("[app_{}] {}", current, inner.tasks[current]);
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.run_next_task(TaskStatus::Ready);
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.run_next_task(TaskStatus::Exited);
}

pub fn print_curent_task_info() {
    TASK_MANAGER.print_task_info();
}
