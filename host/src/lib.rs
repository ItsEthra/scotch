use bincode::{config::standard, error::DecodeError, Decode, Encode};
use std::marker::PhantomData;
use wasmer::{MemorySize, MemoryView};

#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode, M: MemorySize> {
    offset: M::Offset,
    _data: PhantomData<T>,
}

impl<T: Encode + Decode, M: MemorySize> EncodedPtr<T, M> {
    pub fn read(&self, view: MemoryView) -> Result<T, DecodeError> {
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
