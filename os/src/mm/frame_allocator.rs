use alloc::vec::{Vec};
use lazy_static::lazy_static;

use crate::{config::MEMORY_END, info, mm::{PhysAddr, PhysPageNum}, println, sync::up::UPSafeCell};

trait FrameAllocator {
    fn new() -> Self;

    fn alloc(&mut self) -> Option<PhysPageNum>;

    fn dealloc(&mut self, ppn: PhysPageNum);
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> = UPSafeCell::new(FrameAllocatorImpl::new());
}

pub fn init_frame_allocator() {
    unsafe extern "C" {
        safe fn ekernel();
    }

    FRAME_ALLOCATOR.exclusive_access()
        .init(PhysAddr::from(ekernel as usize).ceil(), PhysAddr::from(MEMORY_END).floor());
}

#[inline(never)]
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.exclusive_access().alloc().map(FrameTracker::new)
}

fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
} 

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self { current: 0, end: 0, recycled: Vec::new() }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current < self.end {
            self.current += 1;
            Some((self.current-1).into())
        } else {
            None
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        if ppn >= self.current || self.recycled
            .iter()
            .find(|&v| { v == &ppn })    
            .is_some() {
                panic!("Frame ppn={:#x} has not been allocated!", ppn);
            }
        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

#[derive(Debug)]
pub struct FrameTracker {
    pub ppn: PhysPageNum
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        ppn.get_bytes_array().iter_mut().for_each(|b| *b = 0);
        Self {
            ppn
        }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        info!("frame: {:?}", frame);
        v.push(frame);
    }
    v.clear();

    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        info!("frame: {:?}", frame);
        // v.push(frame);
    }
    drop(v);

    println!("frame_allocator_test passed!");
}
