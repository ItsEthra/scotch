use crate::WasmAllocator;
use parking_lot::RwLock;
use std::{any::TypeId, sync::Arc};

pub use wasmer::{Exports, Instance, RuntimeError, Store, TypedFunction};

pub type StoreRef = Arc<RwLock<Store>>;
pub type WasmAllocRef = Arc<WasmAllocator>;
pub type InstanceRef = Arc<Instance>;

// Don't judge me, its fine because in `WasmPlugin` I check for type ids.
// u128 is weird but i don't know a better way to store it.
pub type CallbackRef = u128;

/// # Safety
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionHandle {
    type Callback;
}

/// # Safety
/// Do not implemented this trait manually.
pub unsafe trait GuestFunctionCreator {
    fn create(
        &self,
        store: StoreRef,
        alloc: WasmAllocRef,
        instance: InstanceRef,
        exports: &Exports,
    ) -> (TypeId, CallbackRef);
}
