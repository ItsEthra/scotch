pub(crate) type PrefixType = u16;

mod encoded;
pub use encoded::*;

mod managed;
pub use managed::*;

mod plugin;
pub use plugin::*;

mod export;
pub use export::*;

mod error;
pub use error::*;

pub use scotch_host_macros::*;
