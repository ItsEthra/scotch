extern crate alloc;

use crate::{MemoryType, PrefixType};
use alloc::borrow::Cow;
use bincode::{
    config::standard,
    error::{DecodeError, EncodeError},
    Decode, Encode,
};
use core::{alloc::Layout, marker::PhantomData, mem::size_of, slice::from_raw_parts};

#[allow(dead_code)]
#[doc(hidden)]
pub struct ManagedPtr<T: Encode + Decode> {
    offset: MemoryType,
    size: usize,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode> ManagedPtr<T> {
    #[inline(always)]
    pub fn offset(&self) -> MemoryType {
        self.offset
    }

    pub fn with_size_by_address(addr: MemoryType) -> Self {
        let mut buf = [0; size_of::<PrefixType>()];
        unsafe {
            buf.as_mut_ptr()
                .copy_from_nonoverlapping(addr as _, size_of::<PrefixType>());
            Self {
                size: PrefixType::from_le_bytes(buf) as _,
                offset: addr,
                _ty: PhantomData,
            }
        }
    }

    pub fn read(&self) -> Result<T, DecodeError> {
        unsafe {
            bincode::decode_from_slice(
                from_raw_parts(
                    (self.offset as usize + size_of::<PrefixType>()) as _,
                    self.size,
                ),
                standard(),
            )
            .map(|(val, _)| val)
        }
    }

    pub fn new(value: &T) -> Result<Self, EncodeError> {
        extern crate alloc;

        let mut buf = [0u8; 64];
        let buf: Cow<[u8]> =
            if let Ok(size) = bincode::encode_into_slice(value, &mut buf, standard()) {
                Cow::Borrowed(&buf[..size])
            } else {
                Cow::Owned(bincode::encode_to_vec(value, standard())?)
            };

        unsafe {
            let ptr = alloc::alloc::alloc(
                Layout::from_size_align(buf.len() + size_of::<PrefixType>(), 1).unwrap(),
            );
            ptr.copy_from_nonoverlapping(
                (buf.len() as PrefixType).to_le_bytes().as_ptr(),
                size_of::<PrefixType>(),
            );
            ptr.add(size_of::<PrefixType>())
                .copy_from_nonoverlapping(buf.as_ptr(), buf.len());

            Ok(Self {
                offset: ptr as MemoryType,
                size: buf.len(),
                _ty: PhantomData,
            })
        }
    }

    pub fn free(self) {
        unsafe {
            alloc::alloc::dealloc(
                self.offset as _,
                Layout::from_size_align(self.size, 1).unwrap(),
            );
        }
    }
}
