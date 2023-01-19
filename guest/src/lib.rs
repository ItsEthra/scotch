#![no_std]

#[cfg(not(feature = "mem64"))]
type MemorySize = u32;
#[cfg(feature = "mem64")]
type MemorySize = u64;

mod encoded;
pub use encoded::*;

pub use scotch_guest_macros::*;
