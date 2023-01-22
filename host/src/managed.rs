use crate::{PrefixType, ScotchHostError};
use bincode::{config::standard, Decode, Encode};
use std::{marker::PhantomData, mem::size_of};
use wasmer::{
    AsStoreMut, FromToNativeWasmType, Instance, Memory32, MemorySize, MemoryView,
    NativeWasmTypeInto,
};

#[doc(hidden)]
pub struct ManagedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    offset: M::Offset,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> ManagedPtr<T, M> {
    pub(crate) fn new(offset: M::Offset) -> Self {
        Self {
            offset,
            _ty: PhantomData,
        }
    }

    pub fn read(&self, view: &MemoryView) -> Result<(T, usize), ScotchHostError> {
        let offset: u64 = self.offset.into();
        let mut buf = [0; size_of::<PrefixType>()];
        view.read(offset, &mut buf)?;

        let len = PrefixType::from_le_bytes(buf) as usize;
        if len < 256 {
            let mut buf = [0; 256];
            view.read(offset + size_of::<PrefixType>() as u64, &mut buf[..len])?;
            Ok((bincode::decode_from_slice(&buf[..len], standard())?.0, len))
        } else {
            let mut buf = Vec::with_capacity(len);
            unsafe {
                buf.set_len(len);
            }

            view.read(offset + size_of::<PrefixType>() as u64, &mut buf[..])?;
            Ok((
                bincode::decode_from_slice(&buf[..], standard())?.0,
                buf.len(),
            ))
        }
    }

    pub fn free_in(
        &self,
        len: usize,
        store: &mut impl AsStoreMut,
        instance: &Instance,
    ) -> Result<(), ScotchHostError> {
        let func = instance
            .exports
            .get_function("__scotch_free")
            .map_err(ScotchHostError::FreeMissing)?;
        func.call(
            store,
            &[
                (self.offset.into() as i32).into(),
                ((len + size_of::<PrefixType>()) as i32).into(),
                1i32.into(),
            ],
        )
        .map(|_| ())
        .map_err(ScotchHostError::FreeFailed)
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
        M::offset_to_native(self.offset)
    }
}
