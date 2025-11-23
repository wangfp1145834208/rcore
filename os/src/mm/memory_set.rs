use core::arch::asm;

use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use bitflags::bitflags;
use lazy_static::lazy_static;
use riscv::{register::satp};

use crate::{config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE}, debug, info, kernel, mm::{FrameTracker, PTEFlags, PageTableEntry, PhysAddr, PhysPageNum, VPNRange, VirtAddr, VirtPageNum, frame_allocator::frame_alloc, page_table::PageTable}, println, sync::up::UPSafeCell};

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(UPSafeCell::new(
        MemorySet::new_kernel()
    ));
}

unsafe extern "C" {
    safe fn stext();
    safe fn etext();

    safe fn srodata();
    safe fn erodata();

    safe fn sdata();
    safe fn edata();

    safe fn sbss_with_stack();
    safe fn ebss();

    safe fn ekernel();
    safe fn strampoline();
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    #[inline(never)]
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: alloc::vec![],
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) -> usize {
        let map_area = MapArea::framed(start_va, end_va, permission);
        let range_size = (usize::from(map_area.vpn_range.get_end()) - usize::from(map_area.vpn_range.get_start())) * PAGE_SIZE;
        self.push(map_area, None);

        range_size
    }

    pub fn map_trampoline(&mut self) {
        let flags = PTEFlags::R | PTEFlags::X;
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(), 
            PhysAddr::from(strampoline as usize).into(), 
            flags,
        );
    }

    #[inline(never)]
    pub fn new_kernel() -> Self {
        let mut memory_set = MemorySet::new_bare();
        memory_set.map_trampoline();

        memory_set.push(MapArea::identical(
            (stext as usize).into(), 
            (etext as usize).into(),
            MapPermission::R | MapPermission::X
        ), None);
        kernel!("mapping .text section success");
        memory_set.push(MapArea::identical(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapPermission::R
        ), None);
        kernel!("mapping .rodata section success");
        memory_set.push(MapArea::identical(
            (sdata as usize).into(), 
            (edata as usize).into(),
           MapPermission::R | MapPermission::W
        ), None);
        kernel!("mapping .data section success");
        memory_set.push(MapArea::identical(
            (sbss_with_stack as usize).into(), 
            (ebss as usize).into(),
           MapPermission::R | MapPermission::W
        ), None);
        kernel!("mapping .bss section success");
        memory_set.push(MapArea::identical(
            (ekernel as usize).into(), 
           MEMORY_END.into(),
           MapPermission::R | MapPermission::W
        ), None);
        kernel!("mapping physical memory success");

        memory_set
    }

    pub fn load_elf(&mut self, elf_data: &[u8]) -> (usize, usize) {
        self.map_trampoline();

        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() { map_perm |= MapPermission::R; }
                if ph_flags.is_write() { map_perm |= MapPermission::W; }
                if ph_flags.is_execute() { map_perm |= MapPermission::X; }
                let map_area = MapArea::framed(
                    start_va,
                    end_va,
                    map_perm,
                );
                max_end_vpn = map_area.vpn_range.get_end();
                self.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize])
                );
            }
        };

        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        // guard page
        let user_stack_bottom = max_end_va + PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        info!("mapped user stack");
        self.push(MapArea::framed(
            user_stack_bottom, user_stack_top, MapPermission::R | MapPermission::W | MapPermission::U
        ), None);
        info!("mapped user stack2");
        self.push(MapArea::framed(
            user_stack_top, user_stack_top, MapPermission::R | MapPermission::W | MapPermission::U
        ), None);

        self.push(MapArea::framed(
            TRAP_CONTEXT.into(), TRAMPOLINE.into(), MapPermission::R | MapPermission::W 
        ), None);

        info!("stack_top: {:#x}, entry: {:#x}", user_stack_top.0, elf.header.pt2.entry_point() as usize);
        (user_stack_top.into(), elf.header.pt2.entry_point() as usize)
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub struct MapArea {
    pub vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        debug!("start_vpn: {:#x}, end_vpn: {:#x}", start_vpn.0, end_vpn.0);
        Self {
            vpn_range: VPNRange::new(start_va.floor(), end_va.ceil()),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm
        }
    }

    pub fn framed(start_va: VirtAddr, end_va: VirtAddr, map_perm: MapPermission) -> Self {
        MapArea::new(start_va, end_va, MapType::Framed, map_perm)
    }

    pub fn identical(start_va: VirtAddr, end_va: VirtAddr, map_perm: MapPermission) -> Self {
        MapArea::new(start_va, end_va, MapType::Identical, map_perm)
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in &self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum = match self.map_type {
            MapType::Identical => vpn.0.into(),
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
                ppn
            }
        };
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in &self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {self.data_frames.remove(&vpn);}
            _ => {}
        }
        page_table.unmap(vpn);
    }

    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        let mut offset = 0usize;
        let size = data.len();
        for vpn in &self.vpn_range {
            if offset >= size {
                break;
            }
            let src = &data[offset..(offset + PAGE_SIZE).min(size)];
            let dst = &mut page_table
                .translate(vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            offset += PAGE_SIZE;
        }
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    info!("mid_text: {:#x}, mid_rodata: {:#x}, mid_data: {:#x}", mid_text.0, mid_rodata.0, mid_data.0);
    assert_eq!(
        kernel_space.page_table.translate(mid_text.floor()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_text.floor()).unwrap().executable(),
        true
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_rodata.floor()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_data.floor()).unwrap().writable(),
        true
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_data.floor()).unwrap().executable(),
        false
    );
    println!("remap_test passed!");
}
