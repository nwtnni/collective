use std::ffi;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct Barrier<'pci> {
    ready: &'pci AtomicU64,
    start: &'pci AtomicBool,
}

impl<'pci> Barrier<'pci> {
    pub const SIZE: usize = crate::CACHE_LINE_SIZE * 2;

    /// Requires first `BARRIER_SIZE` bytes to be zero-initialized.
    pub unsafe fn new(address: *const u8) -> Self {
        let ready = &*address.cast();
        let start = &*address.add(crate::CACHE_LINE_SIZE).cast();
        Self { ready, start }
    }

    pub fn wait(&self, total: ffi::c_int) {
        if self.ready.fetch_add(1, Ordering::AcqRel) + 1 == total as u64 {
            self.start.store(true, Ordering::Release);
        }

        // Spin waiting for all processes to reach barrier
        while !self.start.load(Ordering::Acquire) {}

        // Reset barrier
        if self.ready.fetch_sub(1, Ordering::AcqRel) - 1 == 0 {
            self.start.store(false, Ordering::Release);
        }
    }
}
