use std::{
    error::Error,
    fmt::{self, Display},
};

use bincode::error::{DecodeError, EncodeError};
use wasmer::{ExportError, MemoryAccessError, RuntimeError};

/// Error for everything that can go wrong.
#[derive(Debug)]
pub enum ScotchHostError {
    EncodingFailed(EncodeError),
    DecodingFailed(DecodeError),
    MemoryAccessFailed(MemoryAccessError),
    AllocFailed(RuntimeError),
    FreeFailed(RuntimeError),
    MemoryMissing(ExportError),
    AllocMissing(ExportError),
    FreeMissing(ExportError),
}

impl Display for ScotchHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ScotchHostError {}

macro_rules! impl_from {
    ($target:ident, $($var:ident : $type:ty),*$(,)?) => {
        $(
            impl From<$type> for $target {
                #[inline]
                fn from(v: $type) -> Self {
                    Self::$var(v)
                }
            }
        )*
    }
}

impl_from!(
    ScotchHostError,
    EncodingFailed: EncodeError,
    DecodingFailed: DecodeError,
    MemoryAccessFailed: MemoryAccessError,
);
