#[cfg(feature = "custom")]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "custom")]
use os_common::Heap;
#[cfg(not(feature = "custom"))]
use buddy_system_allocator::LockedHeap;


#[cfg(feature = "custom")]
#[global_allocator]
static GLOBAL_ALLOCATOR: os_common::Heap = Heap::new();
#[cfg(not(feature = "custom"))]
#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(feature = "custom")]
static APPLIED_SIZE: AtomicUsize = AtomicUsize::new(0);
static APPLY_PER_TIME: usize = 4096;
#[cfg(feature = "custom")]
static MAX_APPLY_HEAP: usize = APPLY_PER_TIME * 16;

pub fn get_heap_addr() -> usize {
    unsafe extern "C" {
        safe fn sheap();
    }

    sheap as usize
}

#[cfg(feature = "custom")]
pub fn init_heap() {
    use crate::syscall::sys_mem_apply;

    fn apply_more() -> Option<usize> {
        if APPLIED_SIZE.load(Ordering::Relaxed) >= MAX_APPLY_HEAP {
            None
        } else {
            let applied = sys_mem_apply(get_heap_addr(), APPLY_PER_TIME);
            if applied <= 0 {
                None
            } else {
                APPLIED_SIZE.fetch_add(applied as usize, Ordering::Relaxed);
                Some(applied as usize)
            }
        }
    }

    let applied = apply_more().unwrap();
    GLOBAL_ALLOCATOR.init(
        get_heap_addr(), applied
    );
    GLOBAL_ALLOCATOR.set_apply_more(apply_more);
}

#[cfg(not(feature = "custom"))]
pub fn init_heap() {
    unsafe {
        use crate::syscall::sys_mem_apply;

        let heap_addr = get_heap_addr();
        let applied = sys_mem_apply(heap_addr, APPLY_PER_TIME * 4) as usize;

        GLOBAL_ALLOCATOR.lock().init(heap_addr, applied);
    }
}

