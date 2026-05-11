// SPDX-License-Identifier: GPL-2.0

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use kernel::{
    fs::{File, Kiocb},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    prelude::*,
    str::CString,
};

module! {
    type: RustCounter,
    name: "rustcounter",
    authors: ["Dayyan"],
    description: "rustcounter — an atomic counter character device",
    license: "GPL",
}

static COUNT: AtomicU64 = AtomicU64::new(0);
static CONSUMED: AtomicBool = AtomicBool::new(false);

#[pin_data]
struct RustCounter {
    #[pin]
    _device: MiscDeviceRegistration<CounterDevice>,
}

impl kernel::InPlaceModule for RustCounter {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        pr_info!("module loaded\n");
        let opts = MiscDeviceOptions { name: c"rustcounter" };
        try_pin_init!(Self {
            _device <- MiscDeviceRegistration::register(opts),
        })
    }
}

impl Drop for RustCounter {
    fn drop(&mut self) {
        pr_info!("module unloaded, final = {}\n", COUNT.load(Ordering::SeqCst));
    }
}

struct CounterDevice;

#[vtable]
impl MiscDevice for CounterDevice {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        Ok(KBox::new(CounterDevice, GFP_KERNEL)?.into())
    }

    fn write_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let mut buf: KVec<u8> = KVec::new();
        let n = iov.copy_from_iter_vec(&mut buf, GFP_KERNEL)?;

        let new_count = COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        CONSUMED.store(false, Ordering::SeqCst);
        *kiocb.ki_pos_mut() = 0;

        pr_info!("incremented to {new_count}\n");
        Ok(n)
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        if CONSUMED.swap(true, Ordering::SeqCst) {
            return Ok(0);
        }

        let n = COUNT.load(Ordering::SeqCst);
        let s = CString::try_from_fmt(fmt!("{n}\n"))?;
        iov.simple_read_from_buffer(kiocb.ki_pos_mut(), s.to_bytes())
    }
}
