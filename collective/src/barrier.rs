use std::ffi;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct Barrier<'pci>(&'pci AtomicU64);

impl<'pci> Barrier<'pci> {
    pub const SIZE: usize = crate::CACHE_LINE_SIZE;

    /// Requires first `BARRIER_SIZE` bytes to be zero-initialized.
    pub unsafe fn new(address: *const u8) -> Self {
        Self(&*address.cast())
    }

    pub fn wait(&self, total: ffi::c_int) {
        static EPOCH: AtomicU64 = AtomicU64::new(0);

        let total = total as u64;
        let epoch_before = EPOCH.load(Ordering::Acquire);
        let epoch_after = epoch_before + total;

        if self.0.fetch_add(1, Ordering::AcqRel) + 1 < epoch_after {
            // Spin waiting for all processes to reach barrier
            while self.0.load(Ordering::Acquire) < epoch_after {}
        }

        EPOCH.store(epoch_after, Ordering::Release);
    }
}
