use crate::{PrefixType, ScotchHostError};
use bincode::{config::standard, Decode, Encode};
use std::{marker::PhantomData, mem::size_of};
use wasmer::{FromToNativeWasmType, Memory32, MemorySize, MemoryView, NativeWasmTypeInto};

#[doc(hidden)]
pub struct ManagedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    offset: M::Offset,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> ManagedPtr<T, M> {
    pub fn read(&self, view: &MemoryView) -> Result<T, ScotchHostError> {
        let offset: u64 = self.offset.into();
        let mut buf = [0; size_of::<PrefixType>()];
        view.read(offset, &mut buf)?;

        let len = PrefixType::from_le_bytes(buf) as usize;
        if len < 256 {
            let mut buf = [0; 256];
            view.read(offset + size_of::<PrefixType>() as u64, &mut buf[..len])?;
            Ok(bincode::decode_from_slice(&buf[..len], standard())?.0)
        } else {
            todo!()
        }
    }
}

unsafe impl<T: Encode + Decode, M: MemorySize> FromToNativeWasmType for ManagedPtr<T, M>
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
        unimplemented!("Passing ManagedPtr to guest functions is not allowed")
    }
}
