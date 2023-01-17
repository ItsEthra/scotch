use bincode::{config::standard, error::DecodeError, Decode, Encode};
use std::{marker::PhantomData, string::FromUtf8Error};
use wasmer::{
    FromToNativeWasmType, Memory32, MemorySize, MemoryView, NativeWasmTypeInto, ValueType,
};

#[derive(ValueType)]
#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    offset: M::Offset,
    _data: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> EncodedPtr<T, M> {
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

impl<T: Encode + Decode, M: MemorySize> Clone for EncodedPtr<T, M> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            _data: PhantomData,
        }
    }
}
impl<T: Encode + Decode, M: MemorySize> Copy for EncodedPtr<T, M> {}

#[derive(ValueType)]
#[repr(transparent)]
pub struct EncodedString<M: MemorySize = Memory32> {
    offset: M::Offset,
}

impl<M: MemorySize> EncodedString<M> {
    pub fn read(&self, view: &MemoryView) -> Result<String, FromUtf8Error> {
        let offset: u64 = self.offset.into();
        let mut size = [0, 0];

        // TODO: Handle somehow
        view.read(offset, &mut size).unwrap();

        let len = u16::from_le_bytes(size) as usize;
        let mut data = vec![0; len];

        // TODO: Handle somehow
        view.read(offset + 2, &mut data[..]).unwrap();

        String::from_utf8(data)
    }
}

unsafe impl<M: MemorySize> FromToNativeWasmType for EncodedString<M>
where
    M::Native: NativeWasmTypeInto,
{
    type Native = M::Native;

    #[inline]
    fn from_native(native: Self::Native) -> Self {
        Self {
            offset: M::native_to_offset(native),
        }
    }

    #[inline]
    fn to_native(self) -> Self::Native {
        M::offset_to_native(self.offset)
    }
}
impl<M: MemorySize> Clone for EncodedString<M> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
        }
    }
}
impl<M: MemorySize> Copy for EncodedString<M> {}
