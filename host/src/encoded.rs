use crate::{PrefixType, ScotchHostError};
use bincode::{config::standard, Decode, Encode};
use std::{borrow::Cow, marker::PhantomData, mem::size_of};
use wasmer::{
    AsStoreMut, FromToNativeWasmType, Instance, Memory32, MemorySize, MemoryView,
    NativeWasmTypeInto,
};

#[doc(hidden)]
pub struct EncodedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    pub(crate) offset: M::Offset,
    size: usize,
    _ty: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> EncodedPtr<T, M> {
    pub fn new_in(
        value: &T,
        store: &mut impl AsStoreMut,
        instance: &Instance,
    ) -> Result<Self, ScotchHostError> {
        let mut buf = [0u8; 256];

        // First try encoding to the stack if the object is small,
        // otherwise encode to the heap.
        let buf: Cow<[u8]> =
            if let Ok(size) = bincode::encode_into_slice(value, &mut buf[..], standard()) {
                Cow::Borrowed(&buf[..size])
            } else {
                Cow::Owned(bincode::encode_to_vec(value, standard())?)
            };

        let func = instance
            .exports
            .get_function("__scotch_alloc")
            .map_err(ScotchHostError::AllocMissing)?;
        let out = &func
            .call(
                store,
                &[
                    ((buf.len() + size_of::<PrefixType>()) as i32).into(),
                    1i32.into(),
                ],
            )
            .map_err(ScotchHostError::AllocFailed)?[0];

        #[cfg(feature = "mem64")]
        let ptr = out.unwrap_i64() as u64;
        #[cfg(not(feature = "mem64"))]
        let ptr = out.unwrap_i32() as u64;

        let view = instance
            .exports
            .get_memory("memory")
            .map_err(ScotchHostError::MemoryMissing)?
            .view(store);

        view.write(ptr, &(buf.len() as PrefixType).to_le_bytes())?;
        view.write(ptr + size_of::<PrefixType>() as u64, &buf[..])?;

        if let Ok(offset) = ptr.try_into() {
            Ok(EncodedPtr {
                offset,
                size: buf.len(),
                _ty: PhantomData,
            })
        } else {
            unimplemented!()
        }
    }

    pub fn free_in(
        self,
        mut store: &mut impl AsStoreMut,
        instance: &Instance,
    ) -> Result<(), ScotchHostError> {
        let offset: u64 = self.offset.into();

        let func = instance
            .exports
            .get_function("__scotch_free")
            .map_err(ScotchHostError::FreeMissing)?;
        func.call(
            &mut store,
            &[
                (offset as i32).into(),
                ((self.size + size_of::<PrefixType>()) as i32).into(),
                1i32.into(),
            ],
        )
        .map(|_| ())
        .map_err(ScotchHostError::FreeFailed)
    }

    pub fn read(&self, view: &MemoryView) -> Result<T, ScotchHostError> {
        let offset: u64 = self.offset.into();
        let mut size = [0, 0];

        view.read(offset, &mut size)?;

        let len = u16::from_le_bytes(size) as usize;
        let mut data = vec![0; len];

        view.read(offset + 2, &mut data[..])?;

        Ok(bincode::decode_from_slice(&data[..], standard()).map(|(val, _)| val)?)
    }
}

unsafe impl<T: Encode + Decode, M: MemorySize> FromToNativeWasmType for EncodedPtr<T, M>
where
    M::Native: NativeWasmTypeInto,
{
    type Native = M::Native;

    #[inline]
    fn from_native(_: Self::Native) -> Self {
        unimplemented!("Returning EncodedPtr from guest functions is not allowed")
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
            size: self.size,
        }
    }
}

impl<T: Encode + Decode, M: MemorySize> Copy for EncodedPtr<T, M> {}
