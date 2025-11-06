use alloc::vec::Vec;
use bitflags::*;

use crate::{config::PAGE_SIZE_BITS, info, mm::{PhysPageNum, VirtPageNum, frame_allocator::{FrameTracker, frame_alloc}}};

const PTE_PPN_OFFSET: usize = 10;

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: (ppn.0 << PTE_PPN_OFFSET) | flags.bits() as usize,
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> PTE_PPN_OFFSET).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    fn perm_check(&self, perm: PTEFlags) -> bool {
        (self.flags() & perm) != PTEFlags::empty()
    }

    pub fn is_valid(&self) -> bool {
        self.perm_check(PTEFlags::V)
    }

    pub fn readable(&self) -> bool {
        self.perm_check(PTEFlags::R)
    }

    pub fn writable(&self) -> bool {
        self.perm_check(PTEFlags::W)
    }

    pub fn executable(&self) -> bool {
        self.perm_check(PTEFlags::X)
    }
}

pub struct PageTable {
    root: PhysPageNum,
    frames: Vec<FrameTracker>
}

impl PageTable {
    pub fn new() -> Self {
        let root_frame = frame_alloc().unwrap();
        Self {
            root: root_frame.ppn,
            frames: alloc::vec![root_frame]
        }
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_or_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V); 
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        if let Some(pte) = self.find_pte(vpn) {
            assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
            *pte = PageTableEntry::empty();
        }
    }

    fn find_pte_or_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxes = vpn.indexes();
        let mut ppn = self.root;
        
        for (i, idx) in idxes.into_iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[idx];
            if i == 2 {
                return Some(pte)
            }
            if !pte.is_valid() {
                if let Some(frame) = frame_alloc() {
                    *pte = PageTableEntry::new(frame.ppn, PTEFlags::V); 
                    self.frames.push(frame);
                } else {
                    return None
                }
            }
            ppn = pte.ppn();
        }
        None
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxes = vpn.indexes();
        let mut ppn = self.root;
        for (i, idx) in idxes.into_iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[idx];
            if i == 2 {
                return Some(pte)
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        None
    }

    pub fn token(&self) -> usize {
        (8usize << 60) | self.root.0
    }

    pub fn from_token(satp: usize) -> Self {
        Self {
            root: satp.into(),
            frames: alloc::vec![],
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn)
            .map(|pte| pte.clone())
    }
}
