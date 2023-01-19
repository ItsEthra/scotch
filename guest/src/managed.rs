use crate::{MemoryType, PrefixType};
use bincode::{config::standard, error::EncodeError, Decode, Encode};
use core::{alloc::Layout, marker::PhantomData, mem::size_of};

#[repr(transparent)]
#[allow(dead_code)]
pub struct ManagedPtr<T: Encode + Decode> {
    offset: MemoryType,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode> ManagedPtr<T> {
    pub fn new(value: &T) -> Result<Self, EncodeError> {
        extern crate alloc;

        let mut buf = [0; 64];
        if let Ok(size) = bincode::encode_into_slice(value, &mut buf, standard()) {
            unsafe {
                let shared = alloc::alloc::alloc(Layout::for_value(value));
                shared.copy_from_nonoverlapping(
                    (size as PrefixType).to_le_bytes().as_ptr(),
                    size_of::<PrefixType>(),
                );
                shared
                    .add(size_of::<PrefixType>())
                    .copy_from_nonoverlapping(buf.as_ptr(), size);

                Ok(Self {
                    offset: shared as MemoryType,
                    _ty: PhantomData,
                })
            }
        } else {
            let buf = bincode::encode_to_vec(value, standard())?;
            unsafe {
                let shared = alloc::alloc::alloc(Layout::for_value(value));
                shared.copy_from_nonoverlapping(
                    (buf.len() as PrefixType).to_le_bytes().as_ptr(),
                    size_of::<PrefixType>(),
                );
                shared
                    .add(size_of::<PrefixType>())
                    .copy_from_nonoverlapping(buf.as_ptr(), buf.len());

                Ok(Self {
                    offset: shared as MemoryType,
                    _ty: PhantomData,
                })
            }
        }
    }
}
