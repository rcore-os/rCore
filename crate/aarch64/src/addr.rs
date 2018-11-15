use core::convert::{Into, TryInto};
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use bit_field::BitField;
use usize_conversions::FromUsize;
use ux::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

#[derive(Debug)]
pub struct VirtAddrNotValid(u64);

impl VirtAddr {
    /// Creates a new canonical virtual address.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical. Panics
    /// if the bits in the range 48 to 64 contain data (i.e. are not null and no sign extension).
    pub fn new(addr: u64) -> VirtAddr {
        Self::try_new(addr).expect(
            "invalid virtual address",
        )
    }

    /// Tries to create a new canonical virtual address.
    /// in aarch64, valid virtual address starts with 0x0000 or 0xffff.
    pub fn try_new(addr: u64) -> Result<VirtAddr, VirtAddrNotValid> {
        match addr.get_bits(48..64) {
            0 | 0xffff => Ok(VirtAddr(addr)),      // address is canonical
            other => Err(VirtAddrNotValid(other)),
        }
    }

    pub fn new_unchecked(addr: u64) -> VirtAddr {
        VirtAddr(addr)
    }

    /// Creates a virtual address that points to `0`.
    pub const fn zero() -> VirtAddr {
        VirtAddr(0)
    }

    /// Converts the address to an `u64`.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Creates a virtual address from the given pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        use usize_conversions::FromUsize;

        Self::new(u64::from_usize(ptr as usize))
    }

    /// Converts the address to a raw pointer.
    #[cfg(target_pointer_width = "64")]
    pub fn as_ptr<T>(self) -> *const T {
        use usize_conversions::usize_from;

        usize_from(self.as_u64()) as *const T
    }

    /// Converts the address to a mutable raw pointer.
    #[cfg(target_pointer_width = "64")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    /// Aligns the virtual address upwards to the given alignment.
    ///
    /// See the `align_up` function for more information.
    pub fn align_up<U>(self, align: U) -> Self
    where
        U: Into<u64>,
    {
        VirtAddr(align_up(self.0, align.into()))
    }

    /// Aligns the virtual address downwards to the given alignment.
    ///
    /// See the `align_down` function for more information.
    pub fn align_down<U>(self, align: U) -> Self
    where
        U: Into<u64>,
    {
        VirtAddr(align_down(self.0, align.into()))
    }

    /// Checks whether the virtual address has the demanded alignment.
    pub fn is_aligned<U>(self, align: U) -> bool
    where
        U: Into<u64>,
    {
        self.align_down(align) == self
    }

    /// Returns the 12-bit page offset of this virtual address.
    pub fn page_offset(&self) -> u12 {
        u12::new((self.0 & 0xfff).try_into().unwrap())
    }

    /// Returns the 9-bit level 1 page table index.
    pub fn p1_index(&self) -> u9 {
        u9::new(((self.0 >> 12) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 2 page table index.
    pub fn p2_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 3 page table index.
    pub fn p3_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9 >> 9) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 4 page table index.
    pub fn p4_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9 >> 9 >> 9) & 0o777).try_into().unwrap())
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VirtAddr({:#x})", self.0)
    }
}

impl Add<u64> for VirtAddr {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        VirtAddr::new(self.0 + rhs)
    }
}

impl AddAssign<u64> for VirtAddr {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl Add<usize> for VirtAddr
where
    u64: FromUsize,
{
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        self + u64::from_usize(rhs)
    }
}

impl AddAssign<usize> for VirtAddr
where
    u64: FromUsize,
{
    fn add_assign(&mut self, rhs: usize) {
        self.add_assign(u64::from_usize(rhs))
    }
}

impl Sub<u64> for VirtAddr {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        VirtAddr::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<u64> for VirtAddr {
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl Sub<usize> for VirtAddr
where
    u64: FromUsize,
{
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        self - u64::from_usize(rhs)
    }
}

impl SubAssign<usize> for VirtAddr
where
    u64: FromUsize,
{
    fn sub_assign(&mut self, rhs: usize) {
        self.sub_assign(u64::from_usize(rhs))
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = u64;
    fn sub(self, rhs: VirtAddr) -> Self::Output {
        self.as_u64().checked_sub(rhs.as_u64()).unwrap()
    }
}

/// A passed `u64` was not a valid physical address.
///
/// This means that bits 52 to 64 are not were not all null.
#[derive(Debug)]
pub struct PhysAddrNotValid(u64);

impl PhysAddr {
    /// Creates a new physical address.
    ///
    /// Panics if a bit in the range 52 to 64 is set.
    pub fn new(addr: u64) -> PhysAddr {
        assert_eq!(
            addr.get_bits(52..64),
            0,
            "physical addresses must not have any bits in the range 52 to 64 set"
        );
        PhysAddr(addr)
    }

    /// Tries to create a new physical address.
    ///
    /// Fails if any bits in the range 52 to 64 are set.
    pub fn try_new(addr: u64) -> Result<PhysAddr, PhysAddrNotValid> {
        match addr.get_bits(52..64) {
            0 => Ok(PhysAddr(addr)), // address is valid
            other => Err(PhysAddrNotValid(other)),
        }
    }

    /// Converts the address to an `u64`.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Convenience method for checking if a physical address is null.
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Aligns the physical address upwards to the given alignment.
    ///
    /// See the `align_up` function for more information.
    pub fn align_up<U>(self, align: U) -> Self
    where
        U: Into<u64>,
    {
        PhysAddr(align_up(self.0, align.into()))
    }

    /// Aligns the physical address downwards to the given alignment.
    ///
    /// See the `align_down` function for more information.
    pub fn align_down<U>(self, align: U) -> Self
    where
        U: Into<u64>,
    {
        PhysAddr(align_down(self.0, align.into()))
    }

    /// Checks whether the physical address has the demanded alignment.
    pub fn is_aligned<U>(self, align: U) -> bool
    where
        U: Into<u64>,
    {
        self.align_down(align) == self
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PhysAddr({:#x})", self.0)
    }
}

impl fmt::Binary for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Octal for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<u64> for PhysAddr {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        PhysAddr::new(self.0 + rhs)
    }
}

impl AddAssign<u64> for PhysAddr {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl Add<usize> for PhysAddr
where
    u64: FromUsize,
{
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        self + u64::from_usize(rhs)
    }
}

impl AddAssign<usize> for PhysAddr
where
    u64: FromUsize,
{
    fn add_assign(&mut self, rhs: usize) {
        self.add_assign(u64::from_usize(rhs))
    }
}

impl Sub<u64> for PhysAddr {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        PhysAddr::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<u64> for PhysAddr {
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl Sub<usize> for PhysAddr
where
    u64: FromUsize,
{
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        self - u64::from_usize(rhs)
    }
}

impl SubAssign<usize> for PhysAddr
where
    u64: FromUsize,
{
    fn sub_assign(&mut self, rhs: usize) {
        self.sub_assign(u64::from_usize(rhs))
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = u64;
    fn sub(self, rhs: PhysAddr) -> Self::Output {
        self.as_u64().checked_sub(rhs.as_u64()).unwrap()
    }
}

/// Align address downwards.
///
/// Returns the greatest x with alignment `align` so that x <= addr. The alignment must be
///  a power of 2.
pub fn align_down(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    addr & !(align - 1)
}

/// Align address upwards.
///
/// Returns the smallest x with alignment `align` so that x >= addr. The alignment must be
/// a power of 2.
pub fn align_up(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    let align_mask = align - 1;
    if addr & align_mask == 0 {
        addr // already aligned
    } else {
        (addr | align_mask) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_align_up() {
        // align 1
        assert_eq!(align_up(0, 1), 0);
        assert_eq!(align_up(1234, 1), 1234);
        assert_eq!(align_up(0xffffffffffffffff, 1), 0xffffffffffffffff);
        // align 2
        assert_eq!(align_up(0, 2), 0);
        assert_eq!(align_up(1233, 2), 1234);
        assert_eq!(align_up(0xfffffffffffffffe, 2), 0xfffffffffffffffe);
        // address 0
        assert_eq!(align_up(0, 128), 0);
        assert_eq!(align_up(0, 1), 0);
        assert_eq!(align_up(0, 2), 0);
        assert_eq!(align_up(0, 0x8000000000000000), 0);
    }
}
