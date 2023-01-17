#![no_std]

extern crate alloc;

type MemorySize = u32;

use alloc::{
    string::{FromUtf8Error, String},
    vec::Vec,
};
use bincode::{config::standard, error::DecodeError, Decode, Encode};
use core::{marker::PhantomData, slice::from_raw_parts};

#[repr(transparent)]
pub struct EncodedPtr<T: Encode + Decode> {
    offset: MemorySize,
    _data: PhantomData<T>,
}

impl<T: Encode + Decode> EncodedPtr<T> {
    #[inline]
    pub fn read(&self) -> Result<T, DecodeError> {
        unsafe {
            let mut size = [0, 0];
            (self.offset as *const u8).copy_to_nonoverlapping(&mut size as _, 2);

            let len = u16::from_le_bytes(size) as usize;
            bincode::decode_from_slice(from_raw_parts((self.offset + 2) as _, len), standard())
                .map(|(a, _)| a)
        }
    }
}

#[repr(transparent)]
pub struct EncodedString {
    offset: MemorySize,
}

impl EncodedString {
    pub fn read(&self) -> Result<String, FromUtf8Error> {
        let mut size = [0, 0];

        unsafe { (self.offset as *const u8).copy_to_nonoverlapping(&mut size as _, 2) }

        let len = u16::from_le_bytes(size) as usize;
        let mut data = Vec::with_capacity(len);

        unsafe { ((self.offset + 2) as *const u8).copy_to_nonoverlapping(data.as_mut_ptr(), len) }

        String::from_utf8(data)
    }
}

impl From<String> for EncodedString {
    fn from(value: String) -> Self {
        let len = (value.len() as u16).to_le_bytes();

        let mut data = value.into_bytes();
        data.insert(0, len[0]);
        data.insert(1, len[1]);

        Self {
            offset: Vec::leak(data).as_mut_ptr() as MemorySize,
        }
    }
}

impl Clone for EncodedString {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
        }
    }
}
impl Copy for EncodedString {}
