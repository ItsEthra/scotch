use parking_lot::RwLock;
use std::{any::TypeId, sync::Arc};

pub use wasmer::{Exports, RuntimeError, Store, TypedFunction};

pub type StoreRef = Arc<RwLock<Store>>;

// Don't judge me, its fine because in `WasmPlugin` I check for type ids.
// u128 is weird but i don't know a better way to store it.
pub type CallbackRef = u128;

/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionHandle {
    type Callback;
}

/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionCreator {
    fn create(&self, store: StoreRef, exports: &Exports) -> (TypeId, CallbackRef);
}
