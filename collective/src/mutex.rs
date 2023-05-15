use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct Mutex<'pci>(&'pci AtomicU64);

impl<'pci> Mutex<'pci> {
    pub const SIZE: usize = crate::CACHE_LINE_SIZE;

    const UNLOCKED: u64 = 0;
    const LOCKED: u64 = 1;

    pub unsafe fn new(address: *const u8) -> Self {
        Self(&*address.cast::<AtomicU64>())
    }

    pub fn lock(&self) {
        while self.0.load(Ordering::Acquire) == Self::LOCKED
            || self
                .0
                .compare_exchange(
                    Self::UNLOCKED,
                    Self::LOCKED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {}
    }

    pub fn unlock(&self) {
        self.0.store(Self::UNLOCKED, Ordering::Release);
    }
}
