#![no_std]

/// Pointer size. Can be `u32` or `u64`.
#[cfg(not(feature = "mem64"))]
pub type MemoryType = u32;

/// Pointer size. Can be `u32` or `u64`.
#[cfg(feature = "mem64")]
pub type MemoryType = u64;

type PrefixType = u16;

mod encoded;
pub use encoded::*;

mod managed;
pub use managed::*;

pub use scotch_guest_macros::*;

/// Includes allocation utils for the host. Plugin will not work without it.
/// You need to put it somewhere in your plugin crate.
#[macro_export]
macro_rules! export_alloc {
    () => {
        #[no_mangle]
        extern "C" fn __scotch_alloc(size: u32, align: u32) -> u32 {
            extern crate alloc;
            use alloc::alloc as a;

            unsafe { a::alloc(a::Layout::from_size_align(size as _, align as _).unwrap()) as _ }
        }

        #[no_mangle]
        extern "C" fn __scotch_free(ptr: u32, size: u32, align: u32) {
            extern crate alloc;
            use alloc::alloc as a;

            unsafe {
                a::dealloc(
                    ptr as _,
                    a::Layout::from_size_align(size as _, align as _).unwrap(),
                )
            }
        }
    };
}
