use bincode::{config::standard, error::DecodeError, Decode, Encode};
use core::{marker::PhantomData, slice::from_raw_parts};
use crate::MemorySize;

#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode> {
    offset: MemorySize,
    _data: PhantomData<T>,
}

impl<T: Encode + Decode> EncodedPtr<T> {
    #[inline]
    pub unsafe fn read(&self) -> Result<T, DecodeError> {
        let mut size = [0, 0];
        (self.offset as *const u8).copy_to_nonoverlapping(&mut size as _, 2);

        let len = u16::from_le_bytes(size) as usize;
        bincode::decode_from_slice(from_raw_parts((self.offset + 2) as _, len), standard())
            .map(|(a, _)| a)
    }
}
