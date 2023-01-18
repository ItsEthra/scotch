use bincode::{config::standard, error::DecodeError, Decode, Encode};
use std::marker::PhantomData;
use wasmer::{FromToNativeWasmType, Memory32, MemorySize, MemoryView, NativeWasmTypeInto};

#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode, M: MemorySize = Memory32> {
    offset: M::Offset,
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
    fn from_native(_: Self::Native) -> Self {
        unimplemented!("Returning `EncodedPtr` from guest functions is not allwed")
    }

    #[inline]
    fn to_native(self) -> Self::Native {
        M::offset_to_native(self.offset)
    }
}
