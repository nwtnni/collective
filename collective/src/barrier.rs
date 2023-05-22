use std::ffi;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::metrics;

pub struct Barrier<'pci>(&'pci AtomicU64);

impl<'pci> Barrier<'pci> {
    pub const SIZE: usize = crate::CACHE_LINE_SIZE;

    /// Requires first `BARRIER_SIZE` bytes to be zero-initialized.
    pub unsafe fn new(address: *const u8) -> Self {
        Self(&*address.cast())
    }

    #[cfg_attr(not(feature = "interrupts"), allow(unused_variables))]
    pub fn wait(&self, exclude: ffi::c_int, total: ffi::c_int) {
        static EPOCH: AtomicU64 = AtomicU64::new(0);

        let total = total as u64;
        let epoch_before = EPOCH.load(Ordering::Acquire);
        let epoch_after = epoch_before + total;

        if self.0.fetch_add(1, Ordering::AcqRel) + 1 < epoch_after {
            metrics::time!(metrics::timers::BARRIER, {
                #[cfg(feature = "interrupts")]
                unsafe {
                    use std::os::fd::AsRawFd as _;
                    assert_eq!(
                        libc::read(crate::PCI_FILE.as_raw_fd(), std::ptr::null_mut(), 0),
                        0,
                    );
                }

                // Spin waiting for all processes to reach barrier
                #[cfg(not(feature = "interrupts"))]
                while self.0.load(Ordering::Acquire) < epoch_after {}
            });
        } else {
            #[cfg(feature = "interrupts")]
            for i in 0..total {
                if i as i32 == exclude {
                    continue;
                }
                unsafe {
                    use std::os::fd::AsRawFd as _;
                    assert_eq!(
                        libc::pwrite(
                            crate::PCI_FILE.as_raw_fd(),
                            (i as u16).to_ne_bytes().as_ptr().cast(),
                            std::mem::size_of::<u16>(),
                            0,
                        ),
                        2,
                    );
                }
            }
        }

        EPOCH.store(epoch_after, Ordering::Release);
    }
}
