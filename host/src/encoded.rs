use bincode::{config::standard, error::DecodeError, Decode, Encode};
use std::{marker::PhantomData, mem::size_of};
use wasmer::{
    FromToNativeWasmType, Memory32, MemoryAccessError, MemorySize, MemoryView, NativeWasmTypeInto,
};

use crate::WasmAllocator;

#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    pub(crate) offset: M::Offset,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> EncodedPtr<T, M> {
    #[inline]
    pub(crate) fn new(offset: M::Offset) -> Self {
        Self {
            offset,
            _ty: PhantomData,
        }
    }

    pub fn new_in(
        value: T,
        alloc: &WasmAllocator,
        view: &MemoryView,
    ) -> Result<Self, MemoryAccessError> {
        let mut buf = [0u8; 256];

        type PrefixType = u16;

        // First try encoding to the stack if the object is small,
        // otherwise encode to the heap.
        if let Ok(size) = bincode::encode_into_slice(value, &mut buf[..], standard()) {
            let ptr = alloc
                .alloc((size + size_of::<PrefixType>()) as u32)
                .expect("Allocation failed");
            view.write(ptr as u64, &(size as PrefixType).to_le_bytes())?;
            view.write(ptr as u64 + size_of::<PrefixType>() as u64, &buf[..size])?;

            Ok(EncodedPtr::new(ptr.into()))
        } else {
            todo!()
        }
    }

    pub fn free_in(&self, alloc: &WasmAllocator) {
        let offset: u64 = self.offset.into();
        alloc.free(offset as u32);
    }

    pub fn read(&self, view: &MemoryView) -> Result<T, DecodeError> {
        let offset: u64 = self.offset.into();
        let mut size = [0, 0];

        // TODO: Handle somehow
        _ = view.read(offset, &mut size);

        let len = u16::from_le_bytes(size) as usize;
        let mut data = vec![0; len];

        // TODO: Handle somehow
        _ = view.read(offset + 2, &mut data[..]);

        bincode::decode_from_slice(&data[..], standard()).map(|(val, _)| val)
    }
}

unsafe impl<T: Encode + Decode, M: MemorySize> FromToNativeWasmType for EncodedPtr<T, M>
where
    M::Native: NativeWasmTypeInto,
{
    type Native = M::Native;

    #[inline]
    fn from_native(native: Self::Native) -> Self {
        Self {
            offset: M::native_to_offset(native),
            _ty: PhantomData,
        }
    }

    #[inline]
    fn to_native(self) -> Self::Native {
        M::offset_to_native(self.offset)
    }
}

impl<T: Encode + Decode, M: MemorySize> Clone for EncodedPtr<T, M> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            _ty: PhantomData,
        }
    }
}

impl<T: Encode + Decode, M: MemorySize> Copy for EncodedPtr<T, M> {}
