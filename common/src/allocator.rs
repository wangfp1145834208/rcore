
use core::{
    alloc::{GlobalAlloc},
    cell::{Cell, RefCell},
    fmt::Display,
    ops::Add,
    ptr::null_mut,
    u32,
};

const META_SIZE: usize = 8;

fn align(addr: usize) -> usize {
    let exceed = addr % META_SIZE;
    if exceed == 0 {
        addr
    } else {
        addr + (META_SIZE - exceed)
    }
}

#[repr(C, align(8))]
struct Meta {
    info: u32,

    // 相对指向地址（保证最大可以使用4GB堆）
    rel_ref: u32,
}

impl Meta {
    fn from(addr: usize) -> &'static mut Self {
        unsafe { &mut *(addr as *mut Self) }
    }

    fn addr(&self) -> usize {
        self as *const Self as usize
    }

    fn rel_addr(&self, heap: &HeapInner) -> u32 {
        (self.addr() - heap.token) as u32
    }

    // 保存的是实际可用的内存（已经去掉头部信息）
    fn get_capacity(&self) -> usize {
        (self.info >> 1) as usize
    }

    fn has_occupied(&self) -> bool {
        self.info & 1 == 1
    }

    fn set_free(&mut self) {
        self.info &= u32::MAX - 1;
    }

    fn set_occupy(&mut self) {
        self.info |= 1;
    }

    fn set_capacity(&mut self, capcity: usize) -> &mut Self {
        self.info = ((capcity as u32) << 1) | (self.info & 1);
        self
    }

    fn ref_addr(&self, heap: &HeapInner) -> usize {
        heap.token + self.rel_ref as usize
    }

    fn ref_meta(&self, heap: &HeapInner) -> &'static mut Self {
        self.ref_addr(heap).into()
    }

    fn tail(&self) -> &'static mut Meta {
        (self.addr() + self.get_capacity()).into()
    }

    fn head(&self) -> &'static mut Meta {
        (self.addr() - self.get_capacity()).into()
    }

    fn to_block(&self) -> Block {
        Block {
            head: self.addr().into(),
            tail: self.tail().set_capacity(self.get_capacity()),
        }
    }
}

impl From<usize> for &'static mut Meta {
    fn from(addr: usize) -> Self {
        Meta::from(addr)
    }
}

impl Display for Meta {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "<Meta(addr={:#x})[occupied={}, capacity={}, rel_ref={:#x}",
            self.addr(),
            self.has_occupied(),
            self.get_capacity(),
            self.rel_ref
        )
    }
}

struct Block {
    head: &'static mut Meta,

    // 在Block被释放的时候，tail部分保存的是用户的数据，因此需要进行校验其有效性
    tail: &'static mut Meta,
}

impl Block {
    fn get_capacity(&self) -> usize {
        self.head.get_capacity()
    }

    #[allow(unused)]
    fn has_occupied(&self) -> bool {
        self.head.has_occupied()
    }

    fn set_capacity(&mut self, capacity: usize) -> &mut Self {
        self.head.set_capacity(capacity);
        self.tail = self.head.tail();
        self.tail.set_capacity(capacity);

        self
    }

    fn set_occupied(&mut self) -> &mut Self {
        self.head.set_occupy();
        self.tail.set_occupy();

        self
    }

    fn to_occupied(self) -> Self {
        self.head.set_occupy();
        self.tail.set_occupy();

        self
    }

    fn set_free(&mut self) -> &mut Self {
        self.head.set_free();
        self.tail.set_free();

        self
    }

    fn to_free(self) -> Self {
        self.head.set_free();
        self.tail.set_free();

        self
    }

    fn alloc(&mut self, capacity: usize, heap: &HeapInner) -> Option<usize> {
        let capacity = align(capacity);
        // 需要额外8byte保存元信息，并且保证至少有8byte的可分配空间
        if self.get_capacity() >= capacity + META_SIZE * 2 {
            // 先删除当前块
            self.remove(heap);
            let total_capacity = self.get_capacity();
            let mut pre_block = self.tail.ref_meta(heap).to_block();

            self.set_capacity(capacity).set_occupied();
            let mut left_block = Meta::from(self.tail.addr() + META_SIZE)
                .set_capacity(total_capacity - capacity - META_SIZE)
                .to_block()
                .to_free();

            // 再添加新的块
            pre_block.append(&mut left_block, heap);
            // 或者可以更高效内聚的实现
            Some(self.head.addr())
        } else if self.get_capacity() >= capacity {
            // 将当前块从空闲空间的双向列表里删除
            self.set_occupied().remove(heap);
            Some(self.head.addr())
        } else {
            None
        }
    }

    fn dealloc(self, heap: &HeapInner) {
        let mut last_block = self;
        if let Some(mut right) = last_block.get_freed_right_neighbor(heap) {
            right.remove(heap);
            let total_capacity = last_block.get_capacity() + right.get_capacity() + META_SIZE;
            last_block.set_capacity(total_capacity);
        }
        if let Some(mut left) = last_block.get_freed_left_neighbor(heap) {
            left.remove(heap);
            let total_capacity = last_block.get_capacity() + left.get_capacity() + META_SIZE;
            left.set_capacity(total_capacity);
            last_block = left;
        }

        last_block.set_free();
        heap.root_block().append(&mut last_block, heap);
    }

    fn get_freed_right_neighbor(&self, heap: &HeapInner) -> Option<Block> {
        let right_neighbor_head_addr = self.tail.addr().saturating_add(META_SIZE);
        if !heap.is_valid_addr(right_neighbor_head_addr) {
            return None;
        }
        let right_neighbor_head_meta = Meta::from(right_neighbor_head_addr);
        if right_neighbor_head_meta.has_occupied() {
            return None;
        }

        Some(right_neighbor_head_meta.to_block())
    }

    fn get_freed_left_neighbor(&self, heap: &HeapInner) -> Option<Block> {
        let left_neighbor_tail_addr = self.head.addr().saturating_sub(META_SIZE);
        if !heap.is_valid_addr(left_neighbor_tail_addr) {
            return None;
        }
        let left_neighbor_tail_meta = Meta::from(left_neighbor_tail_addr);
        if left_neighbor_tail_meta.has_occupied() {
            return None;
        }
        // 校验tail的有效性，避免用户数据可能带来的影响
        let capacity = left_neighbor_tail_meta.get_capacity();
        if capacity > left_neighbor_tail_addr {
            return None;
        }
        if !heap.is_valid_addr(left_neighbor_tail_addr - capacity) {
            return None;
        }
        // 校验下一个连接的块是有效的
        let left_neighbor_head_meta = left_neighbor_tail_meta.head();
        if left_neighbor_head_meta.has_occupied()
            || left_neighbor_head_meta.get_capacity() != capacity
            || !heap.is_valid_addr(left_neighbor_head_meta.ref_addr(heap))
            || (left_neighbor_head_meta.ref_addr(heap) != heap.token  // root_block比较特殊
                && left_neighbor_head_meta.ref_meta(heap).has_occupied())
        {
            return None;
        }
        Some(left_neighbor_head_meta.to_block())
    }

    #[allow(dead_code)]
    fn merge_free_neighbor(&mut self, neighbor_block: &mut Block, heap: &HeapInner) -> bool {
        if self.has_occupied() || neighbor_block.has_occupied() {
            return false;
        }
        if self.tail.addr().add(META_SIZE) != neighbor_block.head.addr() {
            return false;
        }

        let total_capcity = self
            .get_capacity()
            .saturating_add(neighbor_block.get_capacity())
            .saturating_add(META_SIZE);
        self.set_capacity(total_capcity).set_free();
        self.head.rel_ref = neighbor_block.head.rel_ref;

        // 更新指向的下一个free block的链接关系
        let next_free_block = self.head.ref_meta(heap).to_block();
        next_free_block.tail.rel_ref = self.head.rel_addr(heap);

        // 更新指向的前一个free block的链接关系
        let pre_free_block = self.tail.ref_meta(heap).to_block();
        pre_free_block.head.rel_ref = self.head.rel_addr(heap);
        true
    }

    fn append(&mut self, block: &mut Block, heap: &HeapInner) {
        let origin_next_block = self.head.ref_meta(heap).to_block();
        block.head.rel_ref = self.head.rel_ref;
        self.head.rel_ref = block.head.rel_addr(heap);
        block.tail.rel_ref = origin_next_block.tail.rel_ref;
        origin_next_block.tail.rel_ref = block.head.rel_addr(heap);
    }

    fn remove(&mut self, heap: &HeapInner) {
        let pre_block = self.tail.ref_meta(heap).to_block();
        let next_block = self.head.ref_meta(heap).to_block();

        pre_block.head.rel_ref = self.head.rel_ref;
        next_block.tail.rel_ref = self.tail.rel_ref;
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "<Block>\n\thead: {}\n\ttail: {}\n</Block>",
            self.head, self.tail
        )
    }
}

struct HeapInner {
    token: usize,
    capacity: Cell<usize>,
    apply_more: Option<fn() -> Option<usize>>,
}

unsafe impl Sync for HeapInner {}

unsafe impl Send for HeapInner {}

impl HeapInner {
    pub const fn new() -> Self {
        Self {
            token: 0,
            capacity: Cell::new(0),
            apply_more: None,
        }
    }

    fn init(&mut self, addr: usize, capacity: usize) {
        if addr % META_SIZE != 0 {
            panic!("invalid addr to init heap");
        }
        self.token = addr;
        self.capacity = Cell::new(capacity);

        // 此处不能为0，需要保证可以找到tail地址
        let root_block = Meta::from(addr)
            .set_capacity(META_SIZE)
            .to_block()
            .to_occupied();
        // root块需要保证head和tail的元信息都被完整保存下来
        let left_capacity = capacity - META_SIZE * 2;
        let next_block = Meta::from(addr + 2 * META_SIZE)
            .set_capacity(left_capacity - META_SIZE)
            .to_block()
            .to_free();

        root_block.head.rel_ref = next_block.head.rel_addr(self);
        root_block.tail.rel_ref = next_block.head.rel_addr(self);
        next_block.head.rel_ref = root_block.head.rel_addr(self);
        next_block.tail.rel_ref = root_block.head.rel_addr(self);
    }

    fn set_apply_more(&mut self, apply_more: fn() -> Option<usize>) {
        self.apply_more.replace(apply_more);
    }

    fn my_alloc(&self, capacity: usize) -> Option<usize> {
        loop {
            if let Some(addr) = self.alloc_inner(capacity) {
                return Some(addr);
            }
            if let Some(apply_more) = self.apply_more
                && let Some(new_space) = apply_more()
            {
                let block = Meta::from(self.end_addr())
                    .set_capacity(new_space - META_SIZE)
                    .to_block()
                    .to_occupied();
                self.capacity.update(|c| c + new_space);
                block.dealloc(self);
            } else {
                return None;
            }
        }
    }

    fn alloc_inner(&self, capacity: usize) -> Option<usize> {
        let root_block = self.root_block();
        let mut cur_block = root_block.head.ref_meta(self).to_block();
        while cur_block.head.addr() != self.token {
            if let Some(addr) = cur_block.alloc(capacity, self) {
                return Some(addr + META_SIZE);
            }
            cur_block = cur_block.head.ref_meta(self).to_block();
        }
        None
    }

    fn my_dealloc(&self, addr: usize) -> Result<(), usize> {
        let addr = addr - META_SIZE;
        if !self.is_valid_addr(addr) || addr % META_SIZE != 0 {
            return Err(1);
        }
        let head = Meta::from(addr);
        if !head.has_occupied() {
            return Err(2);
        }

        let block = head.to_block();
        block.dealloc(self);
        Ok(())
    }

    fn end_addr(&self) -> usize {
        self.token.saturating_add(self.capacity.get())
    }

    fn root_block(&self) -> Block {
        Meta::from(self.token).to_block()
    }

    fn is_valid_addr(&self, addr: usize) -> bool {
        addr >= self.token && addr < self.end_addr()
    }
}

impl Display for HeapInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "<Heap>")?;
        let root_block = self.root_block();
        writeln!(f, "root:\n{}", root_block)?;
        writeln!(f, "left_space:")?;
        let mut cur_block = root_block.head.ref_meta(self).to_block();
        while cur_block.head.addr() != self.token {
            writeln!(f, "{}", cur_block)?;
            cur_block = cur_block.head.ref_meta(self).to_block();
        }
        writeln!(f, "</Heap>")
    }
}

pub struct Heap(RefCell<HeapInner>);

unsafe impl Sync for Heap {}
unsafe impl Send for Heap {}

impl Heap {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Heap(RefCell::new(HeapInner::new()))
    }

    pub fn init(&self, addr: usize, capacity: usize) {
        self.0.borrow_mut().init(addr, capacity);
    }

    pub fn set_apply_more(&self, apply_more: fn() -> Option<usize>) {
        self.0.borrow_mut().set_apply_more(apply_more);
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.size() == 0 {
            return unsafe {
                self.alloc(core::alloc::Layout::from_size_align(1, layout.align()).unwrap())
            };
        }

        if layout.align() > META_SIZE || !layout.align().is_power_of_two() {
            return null_mut();
        }

        if let Some(addr) = self.0.borrow_mut().my_alloc(layout.size()) {
            return addr as *mut u8;
        }

        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: core::alloc::Layout) {
        self.0.borrow_mut().my_dealloc(ptr as usize).unwrap();
    }
}