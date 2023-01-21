extern crate alloc;

use crate::{MemoryType, PrefixType};
use alloc::borrow::Cow;
use bincode::{config::standard, error::EncodeError, Decode, Encode};
use core::{alloc::Layout, marker::PhantomData, mem::size_of};

#[allow(dead_code)]
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

    pub fn new(value: &T) -> Result<Self, EncodeError> {
        extern crate alloc;

        let mut buf = [0; 64];
        let buf: Cow<[u8]> =
            if let Ok(size) = bincode::encode_into_slice(value, &mut buf, standard()) {
                Cow::Borrowed(&buf[..size])
            } else {
                Cow::Owned(bincode::encode_to_vec(value, standard())?)
            };

        unsafe {
            let ptr = alloc::alloc::alloc(Layout::for_value(value));
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
