pub mod heap_allocator;
pub mod address;
pub mod page_table;
pub mod frame_allocator;
pub mod memory_set;

pub use heap_allocator::init_heap;
pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum, VPNRange};
pub use page_table::{PTEFlags, PageTableEntry, translated_byte_buffer};
pub use frame_allocator::{init_frame_allocator, FrameTracker};
pub use memory_set::{KERNEL_SPACE};

use crate::{mm::memory_set::MapPermission, task::with_current_task};

/*
用户的虚拟地址是连续的，但物理地址却可能不连续
*/
pub fn user_data_copy(token: usize, dst: *const u8, mut src: *const u8, len: usize) {
    let buffer = translated_byte_buffer(token, dst, len);
    for d in buffer {
        d.copy_from_slice(unsafe {core::slice::from_raw_parts(src, d.len())});
        src = unsafe { src.add(d.len()) };
    }
}

pub fn mem_apply(va: VirtAddr, size: usize) -> Option<usize> {
    if !va.aligned() {
        return None;
    }
    with_current_task(|tcb| {
        if let Some(tcb) = tcb {
            return Some(tcb.memory_set.insert_framed_area(va, va + size, MapPermission::R | MapPermission::W | MapPermission::U))
        }
        None
    })
}

