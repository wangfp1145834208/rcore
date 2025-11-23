use core::{fmt::Debug, ops::{Add, AddAssign}};

use crate::{config::{PAGE_SIZE, PAGE_SIZE_BITS}, info, mm::PageTableEntry, println};

const PA_WIDTH_SV39: usize = 56;
const VA_WIDTH_SV39: usize = 39;
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;
// sv39每一级包含2**9个页表项，需要9bit存储
const SV39_WIDTH_PER_LEVEL: usize = 9;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VA:{:#x}", self.0)
    }
}

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VPN:{:#x}", self.0)
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PA:{:#x}", self.0)
    }
}

impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PPN:{:#x}", self.0)
    }
}

macro_rules! with_mask {
    ($val:expr, $width:expr) => {
        $val & ((1 << $width) - 1)  
    };
}

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        Self(with_mask!(value, PA_WIDTH_SV39))
    }
}

impl From<usize> for PhysPageNum {
    fn from(value: usize) -> Self {
        Self(with_mask!(value, PPN_WIDTH_SV39))
    }
}

impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        Self(with_mask!(value, VA_WIDTH_SV39))
    }
}

impl From<usize> for VirtPageNum {
    fn from(value: usize) -> Self {
        Self(with_mask!(value, VPN_WIDTH_SV39))
    }
}

impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}

impl From<PhysPageNum> for usize {
    fn from(value: PhysPageNum) -> Self {
        value.0
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        // 高于39位的bit和第39位对齐
        if value.0 >= (1 << (VA_WIDTH_SV39 - 1)) {
            value.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {
            value.0
        }
    }
}

impl From<VirtPageNum> for usize {
    fn from(value: VirtPageNum) -> Self {
        value.0
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<usize> for VirtPageNum {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)        
    }
}

impl AddAssign<usize> for VirtPageNum {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}

impl Add<usize> for PhysPageNum {
    type Output = Self;
    
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl VirtAddr {
    pub fn floor(&self) -> VirtPageNum {
        (self.0 >> PAGE_SIZE_BITS).into()
    }

    pub fn ceil(&self) -> VirtPageNum {
        if self.0 == 0 {
            return 0.into()
        }
        VirtAddr(self.0 - 1).floor() + 1
    }

    pub fn page_offset(&self) -> usize {
        with_mask!(self.0, PAGE_SIZE_BITS)
    }

    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl From<VirtAddr> for VirtPageNum {
    fn from(value: VirtAddr) -> Self {
        value.floor()
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(value: VirtPageNum) -> Self {
        (value.0 << PAGE_SIZE_BITS).into()
    }
}

impl PhysAddr {
    pub fn floor(&self) -> PhysPageNum {
        (self.0 >> PAGE_SIZE_BITS).into()
    }

    pub fn ceil(&self) -> PhysPageNum {
        if self.0 == 0 {
            return 0.into()
        }
        PhysAddr(self.0-1).floor() + 1
    }

    pub fn page_offset(&self) -> usize {
        with_mask!(self.0, PAGE_SIZE_BITS)
    }

    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl From<PhysAddr> for PhysPageNum {
    fn from(value: PhysAddr) -> Self {
        value.floor()
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(value: PhysPageNum) -> Self {
        (value.0 << PAGE_SIZE_BITS).into()
    }
}

impl VirtPageNum {
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut rv = [0; 3];
        for i in (0..3).rev() {
            rv[i] = with_mask!(vpn, SV39_WIDTH_PER_LEVEL);
            vpn >>= SV39_WIDTH_PER_LEVEL;
        }

        rv
    }
}

/*
在项目开启地址空间功能后，即使通过PageTable.translate拿到了ppn，此时使用该ppn的下属方法还是有问题的。
因为该ppn还是会被当做虚拟地址通过satp转换一遍得到“真实”的ppn来取最终的数据。
而为了保证这一点，在开启v39后需要保证MapType是Identical => 即在内核的代码段、数据段才能使用
*/
impl PhysPageNum {
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe {
            core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, PAGE_SIZE / size_of::<PageTableEntry>())
        }
    }

    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe {
            core::slice::from_raw_parts_mut(pa.0 as *mut u8, PAGE_SIZE)
        }
    }

    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe {
            (pa.0 as *mut T).as_mut().unwrap()
        }
    }

    pub fn get_mut_from_back<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (PhysAddr::from(*self).0 + PAGE_SIZE - core::mem::size_of::<T>()).into();
        unsafe {
            (pa.0 as *mut T).as_mut().unwrap()
        }
    }
}

pub trait StepOne {
    fn step(&mut self);
}

impl StepOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Clone, Copy)]
pub struct SimpleRange<T>
where 
    T: StepOne + Copy + PartialEq + PartialOrd + Debug
{
    l: T,
    r: T,
}

impl <T> SimpleRange<T> 
where
    T: StepOne + Copy + PartialEq + PartialOrd + Debug 
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}", start, end);
        Self {l: start, r: end}
    }

    pub fn get_start(&self) -> T {
        self.l
    }

    pub fn get_end(&self) -> T {
        self.r
    }
}

impl <T> IntoIterator for &SimpleRange<T>
where
    T: StepOne + Copy + PartialEq + PartialOrd + Debug 
{
    type IntoIter = SimpleRangeIterator<T>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator {
            cur: self.l,
            end: self.r
        }
    }
}

pub struct SimpleRangeIterator<T> 
where
    T: StepOne + Copy + PartialEq + PartialOrd + Debug 
{
    cur: T,
    end: T,
}

impl <T> Iterator for SimpleRangeIterator<T>
where
    T: StepOne + Copy + PartialEq + PartialOrd + Debug 
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let item = self.cur;
            self.cur.step();
            Some(item)
        } else {
            None
        }
    }
}

pub type VPNRange = SimpleRange<VirtPageNum>;

#[allow(unused)]
pub fn vpn_range_test() {
    // let vpn_range = VPNRange::new(VirtAddr(0x80200000).ceil(), VirtAddr(0x80207000).floor());
    // for vpn in &vpn_range {
    //     info!("vpn: {:?}", vpn);
    // }
    let va = VirtAddr(0x15000);
    let floor_va: VirtAddr = va.floor().into();
    let ceil_va: VirtAddr = va.ceil().into();
    info!("vpn floor: {:#x}, ceil: {:#x}", floor_va.0, ceil_va.0);

    println!("vpn_range_test passed!");
}
